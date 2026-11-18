use std::{net::SocketAddr, time::Duration};

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    mcp::{TOOL_TEXT_MAX_CHARS, ToolInfo, ToolOutput, cap_text, ports::McpGateway},
};
use reqwest13::dns::{Addrs, Name, Resolve, Resolving};
use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParams, ClientConfig, Tool},
    service::RunningService,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::{Map, Value};
use url::Url;

use super::{McpHttp, refusal};
use crate::{NetworkPolicy, guard};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TOOLS: usize = 500;
const DESCRIPTION_MAX_CHARS: usize = 2_000;
const RECONNECT: &str = "la connexion a expiré, reconnectez-vous";

struct GuardedResolver {
    policy: NetworkPolicy,
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let policy = self.policy;
        Box::pin(async move {
            let addresses = guard::guarded_addresses(policy, name.as_str()).await?;
            Ok(Box::new(
                addresses
                    .into_iter()
                    .collect::<Vec<SocketAddr>>()
                    .into_iter(),
            ) as Addrs)
        })
    }
}

pub(super) fn build_transport(
    policy: NetworkPolicy,
    connect_timeout: Duration,
) -> Result<reqwest13::Client, CoreError> {
    reqwest13::Client::builder()
        .redirect(reqwest13::redirect::Policy::none())
        .connect_timeout(connect_timeout)
        .no_proxy()
        .dns_resolver(std::sync::Arc::new(GuardedResolver { policy }))
        .build()
        .map_err(|error| {
            CoreError::Internal(format!(
                "cannot build the MCP HTTP client: {}",
                error.without_url()
            ))
        })
}

impl McpHttp {
    async fn session(
        &self,
        url: &str,
        token: &SecretToken,
    ) -> Result<RunningService<RoleClient, ClientConfig>, CoreError> {
        let target = Url::parse(url)
            .map_err(|_| CoreError::Invalid("l'URL du serveur MCP n'est pas valide".into()))?;
        guard::check_url(self.policy, &target)
            .await
            .map_err(|failure| refusal("le serveur MCP", &failure))?;
        let config = StreamableHttpClientTransportConfig::with_uri(url).auth_header(token.expose());
        let transport = StreamableHttpClientTransport::with_client(self.transport.clone(), config);
        ClientConfig::default()
            .serve(transport)
            .await
            .map_err(|error| remote_error(&error))
    }

    async fn within_deadline<T>(
        future: impl Future<Output = Result<T, CoreError>>,
    ) -> Result<T, CoreError> {
        tokio::time::timeout(OPERATION_TIMEOUT, future)
            .await
            .unwrap_or_else(|_| {
                Err(CoreError::Invalid(
                    "le serveur MCP n'a pas répondu à temps".into(),
                ))
            })
    }
}

fn remote_error(error: &(dyn std::error::Error + 'static)) -> CoreError {
    let mut chain = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        chain.push_str(": ");
        chain.push_str(&cause.to_string());
        source = cause.source();
    }
    let lowered = chain.to_lowercase();
    if lowered.contains("auth required")
        || lowered.contains("401")
        || lowered.contains("unauthorized")
    {
        CoreError::Conflict(RECONNECT.into())
    } else {
        CoreError::Invalid("le serveur MCP a refusé ou n'a pas pu traiter la requête".into())
    }
}

fn tool_info(tool: Tool) -> ToolInfo {
    let (read_only_hint, destructive_hint) = tool
        .annotations
        .as_ref()
        .map(|annotations| (annotations.read_only_hint, annotations.destructive_hint))
        .unwrap_or_default();
    ToolInfo {
        name: tool.name.to_string(),
        description: tool
            .description
            .map(|description| cap_text(&description, DESCRIPTION_MAX_CHARS)),
        input_schema: Value::Object((*tool.input_schema).clone()),
        read_only_hint,
        destructive_hint,
    }
}

pub(crate) fn output_of(result: &Value) -> ToolOutput {
    let text = result["content"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|text| !text.is_empty())
        .or_else(|| {
            result
                .get("structuredContent")
                .filter(|value| !value.is_null())
                .map(Value::to_string)
        })
        .unwrap_or_default();
    ToolOutput {
        text: cap_text(&text, TOOL_TEXT_MAX_CHARS),
        is_error: result["isError"].as_bool().unwrap_or(false),
    }
}

impl McpGateway for McpHttp {
    async fn list_tools(&self, url: &str, token: &SecretToken) -> Result<Vec<ToolInfo>, CoreError> {
        Self::within_deadline(async {
            let session = self.session(url, token).await?;
            let listed = session.peer().list_all_tools().await;
            let _ = session.cancel().await;
            let tools = listed.map_err(|error| remote_error(&error))?;
            Ok(tools.into_iter().take(MAX_TOOLS).map(tool_info).collect())
        })
        .await
    }

    async fn call_tool(
        &self,
        url: &str,
        token: &SecretToken,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolOutput, CoreError> {
        let arguments: Map<String, Value> = match arguments {
            Value::Object(map) => map.clone(),
            Value::Null => Map::new(),
            _ => {
                return Err(CoreError::Invalid(
                    "les arguments d'un outil sont un objet JSON".into(),
                ));
            }
        };
        Self::within_deadline(async {
            let session = self.session(url, token).await?;
            let request = CallToolRequestParams::new(name.to_owned()).with_arguments(arguments);
            let called = session.peer().call_tool(request).await;
            let _ = session.cancel().await;
            let result = called.map_err(|error| remote_error(&error))?;
            let value = serde_json::to_value(&result)
                .map_err(|_| CoreError::Internal("an MCP result cannot be read".into()))?;
            Ok(output_of(&value))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn the_transport_resolver_refuses_private_addresses_at_connection_time() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let strict = build_transport(
            NetworkPolicy {
                allow_insecure_http: true,
                allow_private_networks: false,
            },
            Duration::from_secs(2),
        )
        .unwrap();
        let error = strict
            .get(format!("http://localhost:{port}/"))
            .send()
            .await
            .unwrap_err();
        let mut chain = error.to_string();
        let mut source = std::error::Error::source(&error);
        while let Some(cause) = source {
            chain.push_str(&cause.to_string());
            source = cause.source();
        }
        assert!(chain.contains("network policy blocks"), "{chain}");
    }

    #[test]
    fn text_parts_are_joined_and_the_error_flag_is_kept() {
        let output = output_of(&json!({
            "content": [
                {"type": "text", "text": "a"},
                {"type": "image", "data": "x"},
                {"type": "text", "text": "b"},
            ],
            "isError": true,
        }));
        assert_eq!(output.text, "a\nb");
        assert!(output.is_error);
    }

    #[test]
    fn a_result_without_text_falls_back_to_structured_content() {
        let output = output_of(&json!({"content": [], "structuredContent": {"n": 1}}));
        assert_eq!(output.text, r#"{"n":1}"#);
        assert!(!output.is_error);
    }

    #[test]
    fn the_text_is_capped_at_twelve_thousand_characters() {
        let long = "é".repeat(TOOL_TEXT_MAX_CHARS + 50);
        let output = output_of(&json!({"content": [{"type": "text", "text": long}]}));
        assert_eq!(output.text.chars().count(), TOOL_TEXT_MAX_CHARS + 1);
        assert!(output.text.ends_with('…'));
    }
}
