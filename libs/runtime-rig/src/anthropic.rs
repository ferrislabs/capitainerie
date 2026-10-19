use std::sync::Arc;

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::{SecretToken, Usage},
    provider::AnthropicAuth,
};
use rig_reqwest::reqwest;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tracing::{Instrument, field, info_span};

use crate::usage::truncate_chars;

const VERSION: &str = "2023-06-01";
const ERROR_MAX_CHARS: usize = 300;

#[derive(Clone)]
pub(crate) struct Api {
    http: reqwest::Client,
    base_url: Arc<str>,
    key: SecretToken,
}

#[derive(Debug)]
pub(crate) struct Reply {
    pub content: Vec<Value>,
    pub stop_reason: String,
    pub usage: Usage,
}

impl Reply {
    pub(crate) fn text(&self) -> String {
        self.content
            .iter()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect()
    }

    pub(crate) fn tool_uses(&self) -> impl Iterator<Item = &Value> {
        self.content
            .iter()
            .filter(|block| block["type"] == "tool_use")
    }
}

pub(crate) struct Request<'a> {
    pub model: &'a str,
    pub max_tokens: u32,
    pub system: &'a str,
    pub messages: &'a [Value],
    pub tools: Option<&'a [Value]>,
    pub schema: Option<&'a Value>,
}

impl Request<'_> {
    pub(crate) fn body(&self) -> Value {
        let mut body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": self.system,
            "messages": self.messages,
        });
        if let Some(tools) = self.tools {
            body["tools"] = json!(tools);
            body["tool_choice"] = json!({ "type": "auto" });
        }
        if let Some(schema) = self.schema {
            body["output_config"] = json!({
                "format": { "type": "json_schema", "schema": schema }
            });
        }
        body
    }
}

impl Api {
    pub(crate) fn new(http: reqwest::Client, base_url: &str, key: SecretToken) -> Self {
        Self {
            http,
            base_url: Arc::from(base_url.trim_end_matches('/')),
            key,
        }
    }

    pub(crate) fn messages_request(&self) -> reqwest::RequestBuilder {
        let request = self.http.post(format!("{}/v1/messages", self.base_url));
        authenticate(request, self.key.expose()).header("anthropic-version", VERSION)
    }

    pub(crate) async fn send(&self, request: &Request<'_>) -> Result<Reply, CoreError> {
        let span = info_span!(
            "chat",
            otel.name = format!("chat {}", request.model),
            gen_ai.operation.name = "chat",
            gen_ai.provider.name = "anthropic",
            gen_ai.request.model = request.model,
            gen_ai.usage.input_tokens = field::Empty,
            gen_ai.usage.output_tokens = field::Empty,
            gen_ai.response.finish_reasons = field::Empty,
        );
        let reply = self
            .call(request)
            .instrument(span.clone())
            .await
            .inspect_err(|error| tracing::warn!(parent: &span, %error, "anthropic call failed"))?;
        span.record("gen_ai.usage.input_tokens", reply.usage.input_tokens);
        span.record("gen_ai.usage.output_tokens", reply.usage.output_tokens);
        span.record("gen_ai.response.finish_reasons", reply.stop_reason.as_str());
        Ok(reply)
    }

    async fn call(&self, request: &Request<'_>) -> Result<Reply, CoreError> {
        let request_builder = self.http.post(format!("{}/v1/messages", self.base_url));
        let response = authenticate(request_builder, self.key.expose())
            .header("anthropic-version", VERSION)
            .json(&request.body())
            .send()
            .await
            .map_err(|error| {
                CoreError::Internal(format!("the language model call failed: {error}"))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            CoreError::Internal(format!(
                "the language model response is unreadable: {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(http_failure(status.as_u16(), &body));
        }
        parse(request.model, &body)
    }

    pub(crate) async fn structured<T: DeserializeOwned>(
        &self,
        model: &str,
        system: &str,
        user: &str,
        schema: &Value,
        max_tokens: u32,
        retries: usize,
    ) -> Result<(T, Usage), CoreError> {
        let messages = [json!({ "role": "user", "content": user })];
        let mut total = Usage {
            model: model.to_owned(),
            ..Usage::default()
        };
        let mut last = String::new();
        for _ in 0..=retries {
            let reply = self
                .send(&Request {
                    model,
                    max_tokens,
                    system,
                    messages: &messages,
                    tools: None,
                    schema: Some(schema),
                })
                .await?;
            total.absorb(&reply.usage);
            match serde_json::from_str::<T>(&reply.text()) {
                Ok(parsed) => return Ok((parsed, total)),
                Err(error) => last = error.to_string(),
            }
        }
        Err(CoreError::Invalid(format!(
            "the language model did not return a valid answer after {retries} retries: {last}"
        )))
    }
}

fn parse(model: &str, body: &str) -> Result<Reply, CoreError> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        CoreError::Internal(format!(
            "the language model response is unreadable: {error}"
        ))
    })?;
    let stop_reason = value["stop_reason"]
        .as_str()
        .unwrap_or("end_turn")
        .to_owned();
    match stop_reason.as_str() {
        "refusal" => return Err(refusal(&value)),
        "max_tokens" => {
            return Err(CoreError::Invalid(
                "the language model answer was cut at the output limit".into(),
            ));
        }
        _ => {}
    }
    let content = value["content"].as_array().cloned().unwrap_or_default();
    Ok(Reply {
        content,
        stop_reason,
        usage: Usage {
            model: model.to_owned(),
            input_tokens: value["usage"]["input_tokens"].as_u64().unwrap_or(0),
            output_tokens: value["usage"]["output_tokens"].as_u64().unwrap_or(0),
        },
    })
}

fn refusal(value: &Value) -> CoreError {
    let details = &value["stop_details"];
    let category = details["category"].as_str();
    let explanation = details["explanation"].as_str();
    let mut message = String::from("the language model refused the request");
    if let Some(category) = category {
        message.push_str(&format!(" (category: {category})"));
    }
    if let Some(explanation) = explanation {
        message.push_str(&format!(
            ": {}",
            truncate_chars(explanation, ERROR_MAX_CHARS)
        ));
    }
    CoreError::Invalid(message)
}

pub(crate) fn http_failure(status: u16, body: &str) -> CoreError {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value["error"]["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| truncate_chars(body, ERROR_MAX_CHARS));
    let detail = truncate_chars(&detail, ERROR_MAX_CHARS);
    let message = format!("the language model call returned {status}: {detail}");
    match status {
        408 | 409 | 429 | 500..=599 => CoreError::Internal(message),
        401 | 403 => CoreError::Invalid(format!("{message} (check the provider API key)")),
        _ => CoreError::Invalid(message),
    }
}

pub(crate) fn authenticate(
    request: reqwest::RequestBuilder,
    secret: &str,
) -> reqwest::RequestBuilder {
    match AnthropicAuth::of(secret) {
        AnthropicAuth::OAuthToken => request
            .bearer_auth(secret)
            .header("anthropic-beta", AnthropicAuth::OAUTH_BETA),
        AnthropicAuth::ApiKey => request.header("x-api-key", secret),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn an_oauth_token_is_a_bearer_and_an_api_key_is_x_api_key() {
        let http = reqwest::Client::new();
        let oauth = authenticate(http.get("http://x"), "sk-ant-oat01-t")
            .build()
            .unwrap();
        assert_eq!(oauth.headers()["authorization"], "Bearer sk-ant-oat01-t");
        assert_eq!(oauth.headers()["anthropic-beta"], "oauth-2025-04-20");
        assert!(oauth.headers().get("x-api-key").is_none());
        let key = authenticate(http.get("http://x"), "sk-ant-api03-k")
            .build()
            .unwrap();
        assert_eq!(key.headers()["x-api-key"], "sk-ant-api03-k");
        assert!(key.headers().get("authorization").is_none());
    }

    #[test]
    fn a_request_never_carries_sampling_parameters_or_a_forced_tool() {
        let messages = [json!({ "role": "user", "content": "hi" })];
        let tools = [json!({ "name": "propose_card" })];
        let schema = json!({ "type": "object" });
        let body = Request {
            model: "claude-haiku-5-5",
            max_tokens: 100,
            system: "s",
            messages: &messages,
            tools: Some(&tools),
            schema: Some(&schema),
        }
        .body();
        for forbidden in ["temperature", "top_p", "top_k", "thinking"] {
            assert!(body.get(forbidden).is_none(), "{forbidden}");
        }
        assert_eq!(body["tool_choice"], json!({ "type": "auto" }));
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_refusal_names_its_category() {
        let body = json!({
            "stop_reason": "refusal",
            "stop_details": { "type": "refusal", "category": "cyber", "explanation": "no" },
            "content": [],
            "usage": { "input_tokens": 1, "output_tokens": 0 },
        })
        .to_string();
        let error = parse("m", &body).unwrap_err();
        assert!(
            matches!(&error, CoreError::Invalid(message)
            if message.contains("refused") && message.contains("cyber")),
            "{error:?}"
        );
    }

    #[test]
    fn a_refusal_without_details_is_still_an_error() {
        let body = json!({ "stop_reason": "refusal", "content": [] }).to_string();
        assert!(matches!(parse("m", &body), Err(CoreError::Invalid(_))));
    }

    #[test]
    fn usage_and_text_come_from_the_reply() {
        let body = json!({
            "stop_reason": "end_turn",
            "content": [
                { "type": "thinking", "thinking": "..." },
                { "type": "text", "text": "{\"a\":" },
                { "type": "text", "text": "1}" },
            ],
            "usage": { "input_tokens": 12, "output_tokens": 5 },
        })
        .to_string();
        let reply = parse("claude-haiku-5-5", &body).unwrap();
        assert_eq!(reply.text(), "{\"a\":1}");
        assert_eq!(
            (reply.usage.input_tokens, reply.usage.output_tokens),
            (12, 5)
        );
        assert_eq!(reply.usage.model, "claude-haiku-5-5");
    }

    #[test]
    fn http_failures_are_retryable_only_when_transient() {
        let overloaded = http_failure(
            529,
            r#"{"type":"error","error":{"type":"overloaded_error","message":"busy"}}"#,
        );
        assert!(matches!(overloaded, CoreError::Internal(message) if message.contains("busy")));
        assert!(matches!(http_failure(429, "{}"), CoreError::Internal(_)));
        assert!(matches!(
            http_failure(401, r#"{"error":{"message":"invalid x-api-key"}}"#),
            CoreError::Invalid(message) if message.contains("API key")
        ));
        assert!(matches!(http_failure(400, "oops"), CoreError::Invalid(_)));
    }
}
