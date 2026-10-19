use std::collections::BTreeMap;

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::{SecretToken, Usage},
    conversation::turn::ToolSpec,
};
use rig_reqwest::reqwest;
use serde_json::{Value, json};

use super::{
    driver::{Call, DeltaSink, ModelReply, ModelRequest, Msg, Provider},
    sse::SseDecoder,
};
use crate::{anthropic::http_failure, usage::truncate_chars};

const DONE: &str = "[DONE]";
const ERROR_MAX_CHARS: usize = 300;

pub(crate) struct OpenAiStream {
    pub http: reqwest::Client,
    pub url: String,
    pub key: SecretToken,
    pub model: String,
}

pub(crate) fn body(model: &str, request: &ModelRequest<'_>) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": request.system })];
    for message in request.messages {
        match message {
            Msg::User(text) => messages.push(json!({ "role": "user", "content": text })),
            Msg::Assistant { text, calls } => {
                let mut entry = json!({
                    "role": "assistant",
                    "content": if text.is_empty() { Value::Null } else { json!(text) },
                });
                if !calls.is_empty() {
                    entry["tool_calls"] = json!(
                        calls
                            .iter()
                            .map(|call| json!({
                                "id": call.id,
                                "type": "function",
                                "function": { "name": call.name, "arguments": call.input.to_string() },
                            }))
                            .collect::<Vec<_>>()
                    );
                }
                messages.push(entry);
            }
            Msg::Results(results) => {
                for result in results {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": result.id,
                        "content": result.content,
                    }));
                }
            }
        }
    }
    let mut body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": request.max_tokens,
        "stream": true,
        "stream_options": { "include_usage": true },
    });
    if !request.tools.is_empty() {
        body["tools"] = json!(request.tools.iter().map(tool).collect::<Vec<_>>());
        body["tool_choice"] = json!("auto");
    }
    body
}

fn tool(spec: &ToolSpec) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": spec.name,
            "description": spec.description,
            "parameters": spec.parameters,
        },
    })
}

#[derive(Default)]
struct Pending {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Default)]
struct Assembly {
    reply: ModelReply,
    calls: BTreeMap<u64, Pending>,
    failure: Option<CoreError>,
}

impl Assembly {
    async fn on_event<D: DeltaSink>(&mut self, data: &str, deltas: &mut D) -> bool {
        if data.trim() == DONE {
            return false;
        }
        let Ok(event) = serde_json::from_str::<Value>(data) else {
            return true;
        };
        if let Some(message) = event["error"]["message"].as_str() {
            self.failure = Some(CoreError::Internal(format!(
                "the language model call failed: {}",
                truncate_chars(message, ERROR_MAX_CHARS)
            )));
            return false;
        }
        if let Some(prompt) = event["usage"]["prompt_tokens"].as_u64() {
            self.reply.usage.input_tokens = prompt;
        }
        if let Some(completion) = event["usage"]["completion_tokens"].as_u64() {
            self.reply.usage.output_tokens = completion;
        }
        let delta = &event["choices"][0]["delta"];
        if let Some(text) = delta["content"].as_str().filter(|text| !text.is_empty()) {
            self.reply.text.push_str(text);
            if !deltas.delta(text).await {
                self.reply.stopped = true;
                return false;
            }
        }
        if let Some(calls) = delta["tool_calls"].as_array() {
            for call in calls {
                let index = call["index"].as_u64().unwrap_or(0);
                let pending = self.calls.entry(index).or_default();
                if let Some(id) = call["id"].as_str() {
                    pending.id = id.to_owned();
                }
                if let Some(name) = call["function"]["name"].as_str() {
                    pending.name.push_str(name);
                }
                if let Some(arguments) = call["function"]["arguments"].as_str() {
                    pending.arguments.push_str(arguments);
                }
            }
        }
        true
    }

    fn finish(mut self, model: &str) -> Result<ModelReply, CoreError> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        for (index, pending) in self.calls {
            let input = if pending.arguments.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(&pending.arguments).unwrap_or(Value::Null)
            };
            let id = if pending.id.is_empty() {
                format!("call_{index}")
            } else {
                pending.id
            };
            self.reply.calls.push(Call {
                id,
                name: pending.name,
                input,
            });
        }
        self.reply.usage = Usage {
            model: model.to_owned(),
            ..self.reply.usage
        };
        Ok(self.reply)
    }
}

impl Provider for OpenAiStream {
    async fn stream<D: DeltaSink>(
        &self,
        request: &ModelRequest<'_>,
        deltas: &mut D,
    ) -> Result<ModelReply, CoreError> {
        let mut call = self.http.post(&self.url).json(&body(&self.model, request));
        if !self.key.expose().is_empty() {
            call = call.bearer_auth(self.key.expose());
        }
        let mut response = call.send().await.map_err(|error| {
            CoreError::Internal(format!(
                "the language model call failed: {}",
                error.without_url()
            ))
        })?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(http_failure(status.as_u16(), &text));
        }
        let mut decoder = SseDecoder::default();
        let mut assembly = Assembly::default();
        'read: loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    for data in decoder.push(&chunk) {
                        if !assembly.on_event(&data, deltas).await {
                            break 'read;
                        }
                    }
                }
                Ok(None) => {
                    if let Some(data) = decoder.finish() {
                        assembly.on_event(&data, deltas).await;
                    }
                    break;
                }
                Err(error) => {
                    return Err(CoreError::Internal(format!(
                        "the language model stream broke: {}",
                        error.without_url()
                    )));
                }
            }
        }
        assembly.finish(&self.model)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::captain::driver::ToolResult;

    struct Collect(String);

    impl DeltaSink for Collect {
        async fn delta(&mut self, text: &str) -> bool {
            self.0.push_str(text);
            true
        }
    }

    #[tokio::test]
    async fn text_fragmented_tool_calls_and_usage_are_assembled() {
        let events = [
            json!({"choices": [{"delta": {"role": "assistant", "content": "Je "}}]}),
            json!({"choices": [{"delta": {"content": "regarde."}}]}),
            json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "call_a", "type": "function", "function": {"name": "search_cards", "arguments": ""}}]}}]}),
            json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "function": {"arguments": "{\"query\":"}}]}}]}),
            json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "function": {"arguments": "\"acme\"}"}}]}}]}),
            json!({"choices": [{"delta": {}, "finish_reason": "tool_calls"}]}),
            json!({"choices": [], "usage": {"prompt_tokens": 80, "completion_tokens": 12}}),
        ];
        let mut sink = Collect(String::new());
        let mut assembly = Assembly::default();
        for event in &events {
            assert!(assembly.on_event(&event.to_string(), &mut sink).await);
        }
        assert!(!assembly.on_event("[DONE]", &mut sink).await);

        let reply = assembly.finish("qwen").unwrap();

        assert_eq!(sink.0, "Je regarde.");
        assert_eq!(
            reply.calls,
            [Call {
                id: "call_a".into(),
                name: "search_cards".into(),
                input: json!({"query": "acme"}),
            }]
        );
        assert_eq!(
            (reply.usage.input_tokens, reply.usage.output_tokens),
            (80, 12)
        );
    }

    #[tokio::test]
    async fn a_provider_error_event_fails_the_call() {
        let mut assembly = Assembly::default();
        let mut sink = Collect(String::new());
        assembly
            .on_event(
                &json!({"error": {"message": "quota exceeded"}}).to_string(),
                &mut sink,
            )
            .await;
        assert!(matches!(
            assembly.finish("m"),
            Err(CoreError::Internal(message)) if message.contains("quota")
        ));
    }

    #[test]
    fn the_request_replays_tool_calls_and_results_in_the_openai_shape() {
        let messages = [
            Msg::User("Bonjour".into()),
            Msg::Assistant {
                text: String::new(),
                calls: vec![Call {
                    id: "c1".into(),
                    name: "search_cards".into(),
                    input: json!({"query": "a"}),
                }],
            },
            Msg::Results(vec![ToolResult {
                id: "c1".into(),
                content: "[]".into(),
                is_error: false,
            }]),
        ];
        let tools = [ToolSpec {
            name: "search_cards".into(),
            description: "d".into(),
            parameters: json!({"type": "object"}),
        }];
        let body = body(
            "qwen",
            &ModelRequest {
                system: "s",
                messages: &messages,
                tools: &tools,
                max_tokens: 50,
            },
        );
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][2]["content"], Value::Null);
        assert_eq!(
            body["messages"][2]["tool_calls"][0]["function"]["arguments"],
            "{\"query\":\"a\"}"
        );
        assert_eq!(body["messages"][3]["role"], "tool");
        assert_eq!(body["messages"][3]["tool_call_id"], "c1");
        assert_eq!(body["tools"][0]["function"]["name"], "search_cards");
        assert_eq!(body["stream_options"]["include_usage"], true);
    }
}
