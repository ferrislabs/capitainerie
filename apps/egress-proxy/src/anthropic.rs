use std::sync::Arc;

use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Uri, header},
    response::{IntoResponse, Response},
};
use capitainerie_core::domain::provider::AnthropicAuth;
use serde_json::Value;

use crate::{
    auth,
    credentials::Credentials,
    error::ProxyError,
    forward::{self},
    proxy::Proxy,
};

const CLASS: &str = "anthropic";
const ALLOWED_PATHS: [&str; 2] = ["v1/messages", "v1/messages/count_tokens"];
const KEPT: [HeaderName; 3] = [
    HeaderName::from_static("anthropic-version"),
    header::CONTENT_TYPE,
    header::ACCEPT,
];
const BETA: HeaderName = HeaderName::from_static("anthropic-beta");
const API_KEY: HeaderName = HeaderName::from_static("x-api-key");
const BETA_PREFIXES: [&str; 5] = [
    "prompt-caching-",
    "interleaved-thinking-",
    "fine-grained-tool-streaming-",
    "context-management-",
    "token-efficient-tools-",
];
pub const MAX_REQUEST_BYTES: usize = 10 * 1024 * 1024;
/// Claude Code asks for long outputs (64k and more on current models); the
/// run stays bounded by its turns and its deadline.
pub const MAX_TOKENS_CAP: u64 = 128_000;
const REFUSED_FIELDS: [&str; 2] = ["mcp_servers", "container"];

pub async fn handle<C: Credentials>(
    State(proxy): State<Arc<Proxy<C>>>,
    method: Method,
    Path(path): Path<String>,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    match run(&proxy, &method, &path, &uri, &headers, body).await {
        Ok(response) => response,
        Err(error) => {
            error.log(&method, CLASS);
            error.into_response()
        }
    }
}

/// Only what a task needs to talk to its model: the model of its grant, a
/// bounded answer, and tools that run in the container, never on Anthropic's
/// side. The body is checked, not rewritten.
fn check_body(body: &[u8], model: &str) -> Result<(), &'static str> {
    let value: Value = serde_json::from_slice(body).map_err(|_| "the body is not JSON")?;
    let fields = value.as_object().ok_or("the body is not a JSON object")?;
    if fields.get("model").and_then(Value::as_str) != Some(model) {
        return Err("the model is not the one of this execution");
    }
    match fields.get("max_tokens") {
        None => {}
        Some(tokens) => match tokens.as_u64() {
            Some(tokens) if tokens <= MAX_TOKENS_CAP => {}
            _ => return Err("max_tokens is above the allowed maximum"),
        },
    }
    if REFUSED_FIELDS
        .iter()
        .any(|field| fields.contains_key(*field))
    {
        return Err("mcp_servers and container are not allowed");
    }
    match fields.get("tools") {
        None | Some(Value::Null) => {}
        Some(Value::Array(tools)) if tools.iter().all(is_client_tool) => {}
        Some(_) => return Err("only client tools are allowed"),
    }
    Ok(())
}

fn is_client_tool(tool: &Value) -> bool {
    let Some(tool) = tool.as_object() else {
        return false;
    };
    let typed_as_server = match tool.get("type") {
        None => false,
        Some(kind) => kind.as_str() != Some("custom"),
    };
    !typed_as_server
        && tool.get("name").is_some_and(Value::is_string)
        && tool.contains_key("input_schema")
}

fn beta_allowed(value: &str) -> bool {
    BETA_PREFIXES.iter().any(|prefix| value.starts_with(prefix))
}

/// The client's allowed betas, plus the one the credential requires.
fn allowed_betas(headers: &HeaderMap, required: Option<&'static str>) -> Option<HeaderValue> {
    let kept: Vec<&str> = headers
        .get_all(BETA)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| beta_allowed(value))
        .chain(required)
        .collect();
    if kept.is_empty() {
        return None;
    }
    HeaderValue::from_str(&kept.join(",")).ok()
}

fn forwarded_query(query: Option<&str>) -> &'static str {
    match query {
        Some(query) if query.split('&').any(|pair| pair == "beta=true") => "?beta=true",
        _ => "",
    }
}

async fn run<C: Credentials>(
    proxy: &Proxy<C>,
    method: &Method,
    path: &str,
    uri: &Uri,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response, ProxyError> {
    let token = auth::api_token(headers).ok_or(ProxyError::Unauthorized {
        reason: "no token presented",
    })?;
    let grant = proxy.authenticate(token).await?;
    let run_id = grant.run_id;
    if method != Method::POST || !ALLOWED_PATHS.contains(&path) {
        return Err(ProxyError::forbidden(
            run_id,
            "only POST /v1/messages and /v1/messages/count_tokens are allowed",
        ));
    }
    let permit = proxy.acquire(run_id)?;
    if forward::declared_length(headers).is_some_and(|length| length > MAX_REQUEST_BYTES as u64) {
        return Err(ProxyError::TooLarge { run_id });
    }
    let body: Bytes = axum::body::to_bytes(body, MAX_REQUEST_BYTES)
        .await
        .map_err(|_| ProxyError::TooLarge { run_id })?;
    check_body(&body, &grant.model).map_err(|reason| ProxyError::forbidden(run_id, reason))?;
    let key = proxy
        .credentials
        .anthropic_key(grant.organization_id)
        .await?
        .ok_or(ProxyError::forbidden(
            run_id,
            "no Anthropic key is configured for the organization",
        ))?;

    let mut upstream_headers = forward::copy_allowed(headers, &KEPT);
    let (name, value, required_beta) = match AnthropicAuth::of(key.expose()) {
        AnthropicAuth::OAuthToken => (
            header::AUTHORIZATION,
            format!("Bearer {}", key.expose()),
            Some(AnthropicAuth::OAUTH_BETA),
        ),
        AnthropicAuth::ApiKey => (API_KEY, key.expose().to_owned(), None),
    };
    if let Some(betas) = allowed_betas(headers, required_beta) {
        upstream_headers.insert(BETA, betas);
    }
    let mut credential = HeaderValue::from_str(&value).map_err(|_| {
        ProxyError::forbidden(run_id, "the organization's Anthropic key is unusable")
    })?;
    credential.set_sensitive(true);
    upstream_headers.insert(name, credential);

    let url = format!(
        "{}/{path}{}",
        proxy.config.anthropic_base,
        forwarded_query(uri.query())
    );
    let upstream = proxy
        .http
        .post(url)
        .headers(upstream_headers)
        .body(body)
        .send()
        .await
        .map_err(|error| ProxyError::Upstream(error.without_url().to_string()))?;
    Ok(forward::relay(upstream, permit))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const MODEL: &str = "claude-sonnet-5-5";

    fn check(body: Value) -> Result<(), &'static str> {
        check_body(body.to_string().as_bytes(), MODEL)
    }

    #[test]
    fn a_plain_request_for_the_grant_model_passes() {
        assert_eq!(
            check(json!({
                "model": MODEL,
                "max_tokens": 128_000,
                "messages": [],
                "tools": [
                    {"name": "Bash", "description": "d", "input_schema": {"type": "object"}},
                    {"type": "custom", "name": "Read", "input_schema": {}},
                ],
            })),
            Ok(())
        );
        assert_eq!(check(json!({"model": MODEL})), Ok(()));
    }

    #[test]
    fn another_model_or_none_is_refused() {
        assert!(check(json!({"model": "claude-opus-5"})).is_err());
        assert!(check(json!({"max_tokens": 10})).is_err());
        assert!(check(json!({"model": 3})).is_err());
    }

    #[test]
    fn the_output_is_capped() {
        assert!(check(json!({"model": MODEL, "max_tokens": 128_001})).is_err());
        assert!(check(json!({"model": MODEL, "max_tokens": "10"})).is_err());
        assert!(check(json!({"model": MODEL, "max_tokens": -1})).is_err());
    }

    #[test]
    fn remote_servers_and_containers_are_refused() {
        assert!(check(json!({"model": MODEL, "mcp_servers": []})).is_err());
        assert!(check(json!({"model": MODEL, "container": "c"})).is_err());
    }

    #[test]
    fn server_side_tools_are_refused_whatever_their_version() {
        for kind in [
            "web_search_20250305",
            "web_fetch_20250910",
            "code_execution_20250522",
            "bash_20250124",
            "computer_20250124",
            "text_editor_20250728",
            "mcp_toolset",
        ] {
            let tool = json!({"type": kind, "name": "x", "input_schema": {}});
            assert!(
                check(json!({"model": MODEL, "tools": [tool]})).is_err(),
                "{kind}"
            );
        }
        let typed_only = json!({"type": "web_search_20250305", "name": "web_search"});
        assert!(check(json!({"model": MODEL, "tools": [typed_only]})).is_err());
        let one_bad = json!([
            {"name": "Bash", "input_schema": {}},
            {"type": "web_search_20250305", "name": "web_search"},
        ]);
        assert!(check(json!({"model": MODEL, "tools": one_bad})).is_err());
    }

    #[test]
    fn a_tool_needs_a_name_and_a_schema() {
        assert!(check(json!({"model": MODEL, "tools": [{"name": "Bash"}]})).is_err());
        assert!(check(json!({"model": MODEL, "tools": [{"input_schema": {}}]})).is_err());
        assert!(check(json!({"model": MODEL, "tools": ["Bash"]})).is_err());
        assert!(check(json!({"model": MODEL, "tools": "all"})).is_err());
    }

    #[test]
    fn a_body_that_is_not_a_json_object_is_refused() {
        assert!(check_body(b"not json", MODEL).is_err());
        assert!(check_body(b"[1]", MODEL).is_err());
        assert!(check_body(b"", MODEL).is_err());
    }

    #[test]
    fn only_listed_betas_are_forwarded() {
        let mut headers = HeaderMap::new();
        headers.append(
            BETA,
            HeaderValue::from_static(
                "prompt-caching-2024-07-31, files-api-2025-04-14 ,interleaved-thinking-2025-05-14",
            ),
        );
        headers.append(
            BETA,
            HeaderValue::from_static("token-efficient-tools-2025-02-19"),
        );
        assert_eq!(
            allowed_betas(&headers, None).unwrap(),
            "prompt-caching-2024-07-31,interleaved-thinking-2025-05-14,token-efficient-tools-2025-02-19"
        );
        let mut none = HeaderMap::new();
        none.insert(BETA, HeaderValue::from_static("files-api-2025-04-14"));
        assert_eq!(allowed_betas(&none, None), None);
        assert_eq!(allowed_betas(&HeaderMap::new(), None), None);
        assert_eq!(
            allowed_betas(&none, Some("oauth-2025-04-20")).unwrap(),
            "oauth-2025-04-20"
        );
    }

    #[test]
    fn only_beta_true_survives_in_the_query() {
        assert_eq!(forwarded_query(Some("beta=true")), "?beta=true");
        assert_eq!(forwarded_query(Some("x=1&beta=true&y=2")), "?beta=true");
        assert_eq!(forwarded_query(Some("beta=false")), "");
        assert_eq!(forwarded_query(Some("beta=truee")), "");
        assert_eq!(forwarded_query(Some("foo=bar")), "");
        assert_eq!(forwarded_query(None), "");
    }
}
