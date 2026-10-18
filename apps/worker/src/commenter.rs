use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_core::{CardCommenter, domain::agent::SecretToken};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientConfig},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::{Map, Value, json};

const COMMENT_TOOL: &str = "comment";
const ERROR_MAX_CHARS: usize = 300;

pub struct McpCommenter {
    url: String,
}

impl McpCommenter {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

fn arguments(card_id: CardId, run_id: RunId, body: &str) -> Map<String, Value> {
    let mut arguments = Map::new();
    arguments.insert("card_id".into(), json!(card_id.0.to_string()));
    arguments.insert("body".into(), json!(body));
    arguments.insert("run_id".into(), json!(run_id.0.to_string()));
    arguments
}

fn refusal(result: &Value) -> Option<String> {
    if result["isError"].as_bool() != Some(true) {
        return None;
    }
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    Some(text.chars().take(ERROR_MAX_CHARS).collect())
}

impl CardCommenter for McpCommenter {
    async fn comment(
        &self,
        mcp_token: &SecretToken,
        card_id: CardId,
        run_id: RunId,
        body: &str,
    ) -> Result<(), CoreError> {
        let config = StreamableHttpClientTransportConfig::with_uri(self.url.as_str())
            .auth_header(mcp_token.expose());
        let service = ClientConfig::default()
            .serve(StreamableHttpClientTransport::from_config(config))
            .await
            .map_err(|error| {
                CoreError::Internal(format!("cannot reach the Kanban MCP server: {error}"))
            })?;
        let request = CallToolRequestParams::new(COMMENT_TOOL)
            .with_arguments(arguments(card_id, run_id, body));
        let called = service.peer().call_tool(request).await;
        let _ = service.cancel().await;
        let result = called
            .map_err(|error| CoreError::Internal(format!("the comment call failed: {error}")))?;
        let value = serde_json::to_value(&result)
            .map_err(|error| CoreError::Internal(format!("unreadable MCP result: {error}")))?;
        match refusal(&value) {
            Some(reason) => Err(CoreError::Invalid(format!(
                "the Kanban MCP server refused the comment: {reason}"
            ))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_comment_names_the_card_and_the_run() {
        let card = CardId(uuid::Uuid::from_u128(3));
        let run = RunId(uuid::Uuid::from_u128(1));
        let arguments = arguments(card, run, "salut");
        assert_eq!(arguments["card_id"], card.0.to_string());
        assert_eq!(arguments["run_id"], run.0.to_string());
        assert_eq!(arguments["body"], "salut");
        assert_eq!(arguments.len(), 3);
    }

    #[test]
    fn only_an_error_result_is_a_refusal() {
        assert_eq!(refusal(&json!({"content":[{"text":"{}"}]})), None);
        assert_eq!(
            refusal(&json!({"isError":true,"content":[{"text":"forbidden"}]})).as_deref(),
            Some("forbidden")
        );
    }
}
