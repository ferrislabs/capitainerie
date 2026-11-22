use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Method, Uri, header},
    response::{IntoResponse, Response},
};
use capitainerie_core::domain::{execution::harness::Harness, provider::ProviderKind};
use serde_json::Value;

use crate::{
    auth,
    credentials::Credentials,
    error::ProxyError,
    forward::{self, MAX_OUTPUT_TOKENS_CAP},
    proxy::{self, Proxy},
};

const CLASS: &str = "google";
const ROUTE_PREFIX: &str = "/google/";
const MODELS_PREFIX: &str = "v1beta/models/";
const METHODS: [&str; 2] = ["streamGenerateContent", "generateContent"];
const KEPT: [HeaderName; 2] = [header::CONTENT_TYPE, header::ACCEPT];
const KEY_HEADER: HeaderName = HeaderName::from_static("x-goog-api-key");
const CLIENT_TOOLS_KEY: &str = "functionDeclarations";
const FILE_KEYS: [&str; 2] = ["fileData", "file_data"];
const URI_KEYS: [&str; 2] = ["fileUri", "file_uri"];
const SSE_PAIR: &str = "alt=sse";
const BODY_FIELDS: [&str; 6] = [
    "contents",
    "systemInstruction",
    "generationConfig",
    "tools",
    "toolConfig",
    "safetySettings",
];
const GENERATION_FIELDS: [&str; 12] = [
    "maxOutputTokens",
    "thinkingConfig",
    "temperature",
    "topP",
    "topK",
    "stopSequences",
    "responseMimeType",
    "responseSchema",
    "responseJsonSchema",
    "seed",
    "presencePenalty",
    "frequencyPenalty",
];

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

fn parse_path(path: &str, wire: &str) -> Option<&'static str> {
    let call = path
        .strip_prefix(ROUTE_PREFIX)?
        .strip_prefix(MODELS_PREFIX)?;
    let (model, method) = call.split_once(':')?;
    (model == wire).then_some(())?;
    METHODS.iter().copied().find(|allowed| *allowed == method)
}

fn wants_sse(query: Option<&str>) -> bool {
    query.is_some_and(|query| query.split('&').any(|pair| pair == SSE_PAIR))
}

fn has_key(value: &Value, names: &[&str]) -> bool {
    match value {
        Value::Array(items) => items.iter().any(|item| has_key(item, names)),
        Value::Object(fields) => fields
            .iter()
            .any(|(key, value)| names.contains(&key.as_str()) || has_key(value, names)),
        _ => false,
    }
}

fn is_client_tool(tool: &Value) -> bool {
    tool.as_object()
        .is_some_and(|fields| fields.len() == 1 && fields.contains_key(CLIENT_TOOLS_KEY))
}

fn check_body(body: &[u8]) -> Result<Vec<u8>, &'static str> {
    let value: Value = serde_json::from_slice(body).map_err(|_| "the body is not JSON")?;
    let fields = value.as_object().ok_or("the body is not a JSON object")?;
    if !fields.keys().all(|key| BODY_FIELDS.contains(&key.as_str())) {
        return Err("the body has a field this proxy does not allow");
    }
    match fields.get("generationConfig") {
        None | Some(Value::Null) => {}
        Some(Value::Object(config))
            if config
                .keys()
                .all(|key| GENERATION_FIELDS.contains(&key.as_str())) => {}
        Some(_) => return Err("generationConfig has a field this proxy does not allow"),
    }
    match fields.get("tools") {
        None | Some(Value::Null) => {}
        Some(Value::Array(tools)) if tools.iter().all(is_client_tool) => {}
        Some(_) => return Err("only function declarations are allowed as tools"),
    }
    if let Some(config) = fields.get("generationConfig")
        && let Some(tokens) = config.get("maxOutputTokens")
    {
        match tokens.as_u64() {
            Some(tokens) if tokens <= MAX_OUTPUT_TOKENS_CAP => {}
            _ => return Err("maxOutputTokens is above the allowed maximum"),
        }
    }
    let content = ["contents", "systemInstruction"]
        .iter()
        .filter_map(|key| fields.get(*key));
    if content.into_iter().any(|part| has_key(part, &FILE_KEYS)) {
        return Err("contents may not point at uploaded or remote files");
    }
    if has_key(&value, &URI_KEYS) {
        return Err("file URIs are not allowed");
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
    proxy::ensure_provider(&grant, ProviderKind::Google)?;
    let wire = Harness::Antigravity.wire_model(&grant.model);
    let call = match parse_path(uri.path(), wire) {
        Some(call) if method == Method::POST => call,
        _ => {
            return Err(ProxyError::forbidden(
                run_id,
                "only POST generateContent and streamGenerateContent on the execution's model are allowed",
            ));
        }
    };
    let permit = proxy.acquire(run_id)?;
    let body = forward::read_request(headers, body, run_id).await?;
    let body = check_body(&body).map_err(|reason| ProxyError::bad_request(run_id, reason))?;
    let key = proxy
        .credentials
        .executor_key(
            ProviderKind::Google,
            grant.organization_id,
            grant.initiated_by.as_deref(),
        )
        .await?
        .ok_or(ProxyError::forbidden(
            run_id,
            "no Google key is configured for the run's initiator or the organization",
        ))?;

    let mut upstream_headers = forward::copy_allowed(headers, &KEPT);
    let mut credential = HeaderValue::from_str(key.expose())
        .map_err(|_| ProxyError::forbidden(run_id, "the organization's Google key is unusable"))?;
    credential.set_sensitive(true);
    upstream_headers.insert(KEY_HEADER, credential);

    let mut url = format!("{}/{MODELS_PREFIX}{wire}:{call}", proxy.config.google_base);
    if wants_sse(uri.query()) {
        url.push('?');
        url.push_str(SSE_PAIR);
    }
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

    fn check(body: Value) -> Result<(), &'static str> {
        check_body(body.to_string().as_bytes()).map(|_| ())
    }

    #[test]
    fn fields_outside_what_agy_sends_are_refused() {
        for body in [
            json!({"cached_content": "cachedContents/abc"}),
            json!({"generation_config": {"max_output_tokens": 9_999_999}}),
            json!({"model": "models/gemini-4-argon"}),
            json!({"serviceTier": "priority"}),
            json!({"labels": {"a": "b"}}),
            json!({"generationConfig": {"candidateCount": 8}}),
            json!({"generationConfig": {"max_output_tokens": 9_999_999}}),
            json!({"generationConfig": {"responseModalities": ["IMAGE"]}}),
            json!({"generationConfig": "high"}),
        ] {
            assert!(check(body.clone()).is_err(), "{body}");
        }
    }

    #[test]
    fn the_forwarded_body_keeps_only_one_value_per_key() {
        let duplicated = br#"{"tools":[{"googleSearch":{}}],"tools":[]}"#;
        let forwarded = check_body(duplicated).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&forwarded).unwrap(),
            json!({"tools": []})
        );
        assert_eq!(
            forwarded
                .windows(b"googleSearch".len())
                .position(|w| w == b"googleSearch"),
            None
        );
    }

    #[test]
    fn an_agy_request_passes() {
        let body = json!({
            "contents": [{"role": "user", "parts": [
                {"text": "hi"},
                {"inlineData": {"mimeType": "image/png", "data": "AAAA"}},
            ]}],
            "generationConfig": {"maxOutputTokens": 65536, "thinkingConfig": {"includeThoughts": true}},
            "systemInstruction": {"parts": [{"text": "s"}]},
            "tools": [{"functionDeclarations": [{"name": "run", "parameters": {}}]}],
        });
        assert_eq!(check(body), Ok(()));
        assert_eq!(check(json!({})), Ok(()));
        assert_eq!(check(json!({"tools": null})), Ok(()));
    }

    #[test]
    fn a_body_that_is_not_a_json_object_is_refused() {
        assert!(check_body(b"nope").is_err());
        assert!(check_body(b"[1]").is_err());
        assert!(check_body(b"").is_err());
    }

    #[test]
    fn cached_content_is_refused() {
        assert!(check(json!({"cachedContent": "cachedContents/x"})).is_err());
    }

    #[test]
    fn only_function_declarations_pass_as_tools() {
        for key in [
            "googleSearch",
            "googleSearchRetrieval",
            "codeExecution",
            "urlContext",
            "fileSearch",
            "computerUse",
            "googleMaps",
            "retrieval",
            "mcpServers",
            "enterpriseWebSearch",
            "unknown",
        ] {
            assert!(check(json!({"tools": [{key: {}}]})).is_err(), "{key}");
            let mixed = json!({"tools": [{"functionDeclarations": [], key: {}}]});
            assert!(check(mixed).is_err(), "{key}");
        }
        assert!(check(json!({"tools": [{}]})).is_err());
        assert!(check(json!({"tools": ["functionDeclarations"]})).is_err());
        assert!(check(json!({"tools": "all"})).is_err());
        assert!(check(json!({"tools": {"functionDeclarations": []}})).is_err());
        let one_bad = json!({"tools": [{"functionDeclarations": []}, {"googleSearch": {}}]});
        assert!(check(one_bad).is_err());
    }

    #[test]
    fn the_output_is_capped() {
        let with = |tokens: Value| json!({"generationConfig": {"maxOutputTokens": tokens}});
        assert_eq!(check(with(json!(128_000))), Ok(()));
        assert!(check(with(json!(128_001))).is_err());
        assert!(check(with(json!("10"))).is_err());
        assert!(check(with(json!(-1))).is_err());
        assert!(check(with(json!(1.5))).is_err());
        assert!(check(with(Value::Null)).is_err());
    }

    #[test]
    fn remote_and_uploaded_files_are_refused() {
        let remote = [
            json!({"contents": [{"parts": [{"fileData": {"fileUri": "https://x/", "mimeType": "a/b"}}]}]}),
            json!({"contents": [{"parts": [{"file_data": {"file_uri": "https://x/"}}]}]}),
            json!({"systemInstruction": {"parts": [{"fileData": {}}]}}),
            json!({"systemInstruction": {"parts": [{"file_data": {}}]}}),
            json!({"contents": [{"parts": [{"x": {"fileUri": "gs://b/o"}}]}]}),
            json!({"contents": [{"parts": [{"x": {"file_uri": "gs://b/o"}}]}]}),
            json!({"other": {"fileUri": "gs://b/o"}}),
        ];
        for body in remote {
            assert!(check(body.clone()).is_err(), "{body}");
        }
        let text_only = json!({"contents": [{"parts": [{"text": "fileData and fileUri"}]}]});
        assert_eq!(check(text_only), Ok(()));
    }

    #[test]
    fn the_path_must_be_exactly_the_grants_model_and_a_generate_call() {
        let wire = "gemini-3.1-pro-preview";
        let ok = |path: &str| parse_path(path, wire);
        assert_eq!(
            ok("/google/v1beta/models/gemini-3.1-pro-preview:streamGenerateContent"),
            Some("streamGenerateContent")
        );
        assert_eq!(
            ok("/google/v1beta/models/gemini-3.1-pro-preview:generateContent"),
            Some("generateContent")
        );
        for path in [
            "/google/v1beta/models/gemini-3.1-pro:streamGenerateContent",
            "/google/v1beta/models/gemini-3.1-pro-preview:countTokens",
            "/google/v1beta/models/gemini-3.1-pro-preview",
            "/google/v1beta/models/gemini-3.1-pro-preview:",
            "/google/v1beta/models/gemini-3.1-pro-preview:generateContent/x",
            "/google/v1beta/models/gemini-3.1-pro-preview:generateContent/",
            "/google/v1beta/models/gemini-3.1-pro-preview%3AgenerateContent",
            "/google/v1beta/models/x/../gemini-3.1-pro-preview:generateContent",
            "/google/v1beta/models/gemini-3.1-pro-preview:generateContent:x",
            "/google/v1beta/tunedModels/gemini-3.1-pro-preview:generateContent",
            "/google/v1/models/gemini-3.1-pro-preview:generateContent",
            "/google/v1beta/models",
            "/google/",
        ] {
            assert_eq!(ok(path), None, "{path}");
        }
    }

    #[test]
    fn only_alt_sse_is_kept_from_the_query() {
        assert!(wants_sse(Some("alt=sse")));
        assert!(wants_sse(Some("key=x&alt=sse")));
        assert!(!wants_sse(Some("alt=json")));
        assert!(!wants_sse(Some("alt=sse2")));
        assert!(!wants_sse(None));
    }
}
