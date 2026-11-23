use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Method, Uri, header},
    response::{IntoResponse, Response},
};
use capitainerie_core::domain::{execution::harness::Harness, provider::ProviderKind};
use serde_json::{Map, Value};

use crate::{
    auth,
    credentials::Credentials,
    error::ProxyError,
    forward::{self, MAX_OUTPUT_TOKENS_CAP},
    proxy::{self, Proxy},
};

const CLASS: &str = "mistral";
const ROUTE_PATH: &str = "/mistral/v1/chat/completions";
const UPSTREAM_PATH: &str = "/v1/chat/completions";
const KEPT: [HeaderName; 2] = [header::CONTENT_TYPE, header::ACCEPT];
const INLINE_IMAGE_PREFIX: &str = "data:";
const BODY_FIELDS: [&str; 15] = [
    "model",
    "messages",
    "stream",
    "stream_options",
    "temperature",
    "top_p",
    "max_tokens",
    "tools",
    "tool_choice",
    "parallel_tool_calls",
    "random_seed",
    "stop",
    "reasoning_effort",
    "prompt_mode",
    "response_format",
];
const ROLES: [&str; 4] = ["system", "user", "assistant", "tool"];
const MESSAGE_FIELDS: [&str; 7] = [
    "role",
    "content",
    "tool_calls",
    "tool_call_id",
    "name",
    "prefix",
    "reasoning_content",
];
const RESPONSE_FORMATS: [&str; 3] = ["json_object", "text", "json_schema"];
const FUNCTION_FIELDS: [&str; 4] = ["name", "description", "parameters", "strict"];

pub async fn handle<C: Credentials>(
    State(proxy): State<Arc<Proxy<C>>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    match run(&proxy, &method, &uri, &headers, body).await {
        Ok(response) => response,
        Err(error) => {
            error.log(&method, CLASS);
            error.into_response()
        }
    }
}

fn only_keys(fields: &Map<String, Value>, allowed: &[&str]) -> bool {
    fields.keys().all(|key| allowed.contains(&key.as_str()))
}

fn check_parts(parts: &[Value]) -> Result<(), &'static str> {
    parts.iter().try_for_each(check_part)
}

fn check_part(part: &Value) -> Result<(), &'static str> {
    let fields = part.as_object().ok_or("a content part is not an object")?;
    match fields.get("type").and_then(Value::as_str) {
        Some("text") if only_keys(fields, &["type", "text"]) => Ok(()),
        Some("thinking") if only_keys(fields, &["type", "thinking", "closed"]) => {
            match fields.get("thinking") {
                None | Some(Value::Null) => Ok(()),
                Some(Value::Array(inner)) => check_parts(inner),
                Some(_) => Err("a thinking part is malformed"),
            }
        }
        Some("image_url") if only_keys(fields, &["type", "image_url"]) => {
            match fields.get("image_url") {
                Some(Value::String(url)) if url.starts_with(INLINE_IMAGE_PREFIX) => Ok(()),
                Some(Value::Object(image))
                    if only_keys(image, &["url", "detail"])
                        && image
                            .get("url")
                            .and_then(Value::as_str)
                            .is_some_and(|url| url.starts_with(INLINE_IMAGE_PREFIX)) =>
                {
                    Ok(())
                }
                _ => Err("images must be inline data URLs"),
            }
        }
        _ => Err("only text, thinking and inline image parts are allowed"),
    }
}

fn check_content(content: &Value) -> Result<(), &'static str> {
    match content {
        Value::Null | Value::String(_) => Ok(()),
        Value::Array(parts) => check_parts(parts),
        _ => Err("content must be a string, null or a list of parts"),
    }
}

fn check_message(message: &Value) -> Result<(), &'static str> {
    let fields = message.as_object().ok_or("a message is not an object")?;
    if !only_keys(fields, &MESSAGE_FIELDS) {
        return Err("a message has a field this proxy does not allow");
    }
    match fields.get("role").and_then(Value::as_str) {
        Some(role) if ROLES.contains(&role) => {}
        _ => return Err("a message has an unknown role"),
    }
    for key in ["content", "reasoning_content"] {
        if let Some(content) = fields.get(key) {
            check_content(content)?;
        }
    }
    match fields.get("tool_calls") {
        None | Some(Value::Null) => {}
        Some(Value::Array(calls)) if calls.iter().all(Value::is_object) => {}
        Some(_) => return Err("tool_calls is malformed"),
    }
    Ok(())
}

fn check_tool(tool: &Value) -> Result<(), &'static str> {
    let message = "only function tools are allowed";
    let fields = tool.as_object().ok_or(message)?;
    if !only_keys(fields, &["type", "function"])
        || fields.get("type").and_then(Value::as_str) != Some("function")
    {
        return Err(message);
    }
    let function = fields
        .get("function")
        .and_then(Value::as_object)
        .ok_or(message)?;
    if only_keys(function, &FUNCTION_FIELDS) && function.get("name").is_some_and(Value::is_string) {
        Ok(())
    } else {
        Err(message)
    }
}

fn check_body(body: &[u8], wire_model: &str) -> Result<Vec<u8>, &'static str> {
    let mut value: Value = serde_json::from_slice(body).map_err(|_| "the body is not JSON")?;
    let fields = value
        .as_object_mut()
        .ok_or("the body is not a JSON object")?;
    if !only_keys(fields, &BODY_FIELDS) {
        return Err("the body has a field this proxy does not allow");
    }
    if fields.get("model").and_then(Value::as_str) != Some(wire_model) {
        return Err("the model is not the one of this execution");
    }
    match fields.get("messages") {
        Some(Value::Array(messages)) => messages.iter().try_for_each(check_message)?,
        _ => return Err("messages must be a list"),
    }
    match fields.get("max_tokens") {
        None => {
            fields.insert("max_tokens".to_owned(), MAX_OUTPUT_TOKENS_CAP.into());
        }
        Some(tokens) => match tokens.as_u64() {
            Some(tokens) if tokens <= MAX_OUTPUT_TOKENS_CAP => {}
            _ => return Err("max_tokens is above the allowed maximum"),
        },
    }
    match fields.get("stream_options") {
        None | Some(Value::Null) => {}
        Some(Value::Object(options))
            if only_keys(options, &["include_usage"])
                && options.get("include_usage").is_none_or(Value::is_boolean) => {}
        Some(_) => return Err("stream_options only allows include_usage"),
    }
    match fields.get("tools") {
        None | Some(Value::Null) => {}
        Some(Value::Array(tools)) => tools.iter().try_for_each(check_tool)?,
        Some(_) => return Err("only function tools are allowed"),
    }
    match fields.get("response_format") {
        None | Some(Value::Null) => {}
        Some(Value::Object(format))
            if only_keys(format, &["type", "json_schema"])
                && format
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| RESPONSE_FORMATS.contains(&kind)) => {}
        Some(_) => return Err("only json_object, text and json_schema formats are allowed"),
    }
    serde_json::to_vec(&value).map_err(|_| "the body cannot be re-encoded")
}

async fn run<C: Credentials>(
    proxy: &Proxy<C>,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response, ProxyError> {
    let token = auth::api_token(headers).ok_or(ProxyError::Unauthorized {
        reason: "no token presented",
    })?;
    let grant = proxy.authenticate(token).await?;
    let run_id = grant.run_id;
    proxy::ensure_provider(&grant, ProviderKind::Mistral)?;
    if method != Method::POST || uri.path() != ROUTE_PATH || uri.query().is_some() {
        return Err(ProxyError::forbidden(
            run_id,
            "only POST /v1/chat/completions without a query is allowed",
        ));
    }
    let wire = Harness::MistralVibe.wire_model(&grant.model);
    let permit = proxy.acquire(run_id)?;
    let body = forward::read_request(headers, body, run_id).await?;
    let body = check_body(&body, wire).map_err(|reason| ProxyError::bad_request(run_id, reason))?;
    let key = proxy
        .credentials
        .executor_key(
            ProviderKind::Mistral,
            grant.organization_id,
            grant.initiated_by.as_deref(),
        )
        .await?
        .ok_or(ProxyError::forbidden(
            run_id,
            "no Mistral key is configured for the run's initiator or the organization",
        ))?;

    let mut upstream_headers = forward::copy_allowed(headers, &KEPT);
    let mut credential = HeaderValue::from_str(&format!("Bearer {}", key.expose()))
        .map_err(|_| ProxyError::forbidden(run_id, "the organization's Mistral key is unusable"))?;
    credential.set_sensitive(true);
    upstream_headers.insert(header::AUTHORIZATION, credential);

    let url = format!("{}{UPSTREAM_PATH}", proxy.config.mistral_base);
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

    const WIRE: &str = "mistral-large-4-0";
    const FIXTURE: &str = include_str!("../tests/fixtures/vibe-request-body.json");

    fn check(body: Value) -> Result<(), &'static str> {
        check_body(body.to_string().as_bytes(), WIRE).map(|_| ())
    }

    fn with(field: &str, value: Value) -> Value {
        let mut body: Value = serde_json::from_str(FIXTURE).unwrap();
        body[field] = value;
        body
    }

    fn message(content: Value) -> Value {
        with("messages", json!([{"role": "user", "content": content}]))
    }

    #[test]
    fn the_real_vibe_body_passes_and_is_reserialized() {
        let out = check_body(FIXTURE.as_bytes(), WIRE).unwrap();
        let mut expected = serde_json::from_str::<Value>(FIXTURE).unwrap();
        expected["max_tokens"] = MAX_OUTPUT_TOKENS_CAP.into();
        assert_eq!(serde_json::from_slice::<Value>(&out).unwrap(), expected);
    }

    #[test]
    fn duplicate_keys_collapse_in_the_forwarded_body() {
        let raw = r#"{"model":"mistral-large-4-0","messages":[],"stream":false,"stream":true}"#;
        let out = check_body(raw.as_bytes(), WIRE).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap().matches("stream\"").count(),
            1
        );
    }

    #[test]
    fn a_body_that_is_not_a_json_object_is_refused() {
        assert!(check_body(b"nope", WIRE).is_err());
        assert!(check_body(b"[1]", WIRE).is_err());
        assert!(check_body(b"", WIRE).is_err());
    }

    #[test]
    fn the_model_must_be_the_wire_model() {
        assert!(check(with("model", json!("mistral-medium-3.5"))).is_err());
        assert!(check(with("model", json!("mistral-vibe-cli-latest"))).is_err());
        assert!(check(with("model", Value::Null)).is_err());
        assert!(check_body(FIXTURE.as_bytes(), "mistral-vibe-cli-latest").is_err());
    }

    #[test]
    fn unknown_top_level_fields_are_refused() {
        for key in [
            "n",
            "user",
            "metadata",
            "safe_prompt",
            "presence_penalty",
            "x",
        ] {
            assert!(check(with(key, json!(1))).is_err(), "{key}");
        }
        for key in ["temperature", "top_p", "random_seed", "stop", "tool_choice"] {
            assert_eq!(check(with(key, json!("auto"))), Ok(()), "{key}");
        }
    }

    #[test]
    fn the_output_is_capped() {
        assert_eq!(check(with("max_tokens", json!(128_000))), Ok(()));
        assert!(check(with("max_tokens", json!(128_001))).is_err());
        assert!(check(with("max_tokens", json!("10"))).is_err());
        assert!(check(with("max_tokens", json!(-1))).is_err());
    }

    #[test]
    fn a_body_without_max_tokens_is_capped() {
        let forwarded: Value =
            serde_json::from_slice(&check_body(FIXTURE.as_bytes(), WIRE).unwrap()).unwrap();
        assert_eq!(forwarded["max_tokens"], MAX_OUTPUT_TOKENS_CAP);
        assert!(check(with("max_tokens", Value::Null)).is_err());
    }

    #[test]
    fn stream_options_only_carry_include_usage() {
        assert_eq!(
            check(with("stream_options", json!({"include_usage": false}))),
            Ok(())
        );
        assert!(check(with("stream_options", json!({"include_usage": 1}))).is_err());
        assert!(check(with("stream_options", json!({"other": true}))).is_err());
        assert!(check(with("stream_options", json!("x"))).is_err());
    }

    #[test]
    fn only_plain_function_tools_pass() {
        let ok = json!([{"type": "function", "function": {"name": "a", "description": "d", "parameters": {}, "strict": true}}]);
        assert_eq!(check(with("tools", ok)), Ok(()));
        for tools in [
            json!([{"type": "web_search"}]),
            json!([{"type": "function"}]),
            json!([{"type": "function", "function": {}}]),
            json!([{"type": "function", "function": {"name": "a", "url": "x"}}]),
            json!([{"type": "function", "function": {"name": "a"}, "extra": 1}]),
            json!([{"function": {"name": "a"}}]),
            json!(["function"]),
            json!({"type": "function"}),
        ] {
            assert!(check(with("tools", tools.clone())).is_err(), "{tools}");
        }
    }

    #[test]
    fn messages_are_checked() {
        assert!(check(with("messages", json!("x"))).is_err());
        assert!(check(with("messages", json!(["x"]))).is_err());
        assert!(
            check(with(
                "messages",
                json!([{"role": "developer", "content": "x"}])
            ))
            .is_err()
        );
        assert!(check(with("messages", json!([{"content": "x"}]))).is_err());
        assert!(
            check(with(
                "messages",
                json!([{"role": "user", "content": "x", "x": 1}])
            ))
            .is_err()
        );
        assert!(check(with("messages", json!([{"role": "user", "content": 1}]))).is_err());
        assert!(
            check(with(
                "messages",
                json!([{"role": "assistant", "tool_calls": "x"}])
            ))
            .is_err()
        );
        let full = json!([
            {"role": "assistant", "content": null, "tool_calls": [{"id": "1", "type": "function", "function": {"name": "a", "arguments": "{}"}}], "prefix": false, "reasoning_content": "r"},
            {"role": "tool", "tool_call_id": "1", "name": "a", "content": "ok"},
        ]);
        assert_eq!(check(with("messages", full)), Ok(()));
    }

    #[test]
    fn content_parts_are_text_thinking_or_inline_images() {
        let ok = json!([
            {"type": "text", "text": "hi"},
            {"type": "thinking", "thinking": [{"type": "text", "text": "t"}], "closed": true},
            {"type": "image_url", "image_url": "data:image/png;base64,AAAA"},
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA", "detail": "low"}},
        ]);
        assert_eq!(check(message(ok)), Ok(()));
        assert_eq!(check(message(Value::Null)), Ok(()));
    }

    #[test]
    fn remote_and_file_parts_are_refused() {
        for part in [
            json!({"type": "image_url", "image_url": "https://attacker.example/x.png"}),
            json!({"type": "image_url", "image_url": {"url": "http://169.254.169.254/"}}),
            json!({"type": "image_url", "image_url": {"url": "data:image/png;base64,AA", "x": 1}}),
            json!({"type": "image_url", "image_url": {}}),
            json!({"type": "image_url"}),
            json!({"type": "document_url", "document_url": "https://x/"}),
            json!({"type": "file", "file_id": "f"}),
            json!({"type": "input_audio", "input_audio": "AAAA"}),
            json!({"type": "reference", "reference_ids": [1]}),
            json!({"type": "text", "text": "x", "extra": 1}),
            json!({"type": "thinking", "thinking": [{"type": "document_url", "document_url": "https://x/"}]}),
            json!({"text": "no type"}),
            json!("text"),
        ] {
            assert!(check(message(json!([part.clone()]))).is_err(), "{part}");
        }
        assert!(check(message(json!(1))).is_err());
        let reasoning = with(
            "messages",
            json!([{"role": "assistant", "reasoning_content": [{"type": "document_url"}]}]),
        );
        assert!(check(reasoning).is_err());
    }

    #[test]
    fn response_format_is_limited() {
        for kind in ["json_object", "text", "json_schema"] {
            assert_eq!(
                check(with("response_format", json!({"type": kind}))),
                Ok(()),
                "{kind}"
            );
        }
        assert!(check(with("response_format", json!({"type": "grammar"}))).is_err());
        assert!(check(with("response_format", json!({"type": "text", "x": 1}))).is_err());
        assert!(check(with("response_format", json!("text"))).is_err());
    }
}
