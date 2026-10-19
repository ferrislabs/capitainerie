use std::collections::BTreeMap;

use capitainerie_common::CoreError;
use capitainerie_core::domain::conversation::turn::ToolSpec;
use serde_json::{Value, json};

use super::{
    driver::{Call, DeltaSink, ModelReply, ModelRequest, Msg, Provider},
    sse::{
        MAX_ERROR_BODY_BYTES, MAX_REPLY_BYTES, MAX_TOOL_JSON_BYTES, SseDecoder, overflow,
        read_capped_text,
    },
};
use crate::{
    anthropic::{Api, http_failure},
    usage::truncate_chars,
};

const ERROR_MAX_CHARS: usize = 300;

pub(crate) struct AnthropicStream {
    pub api: Api,
    pub model: String,
}

pub(crate) fn body(model: &str, request: &ModelRequest<'_>) -> Value {
    let mut body = json!({
        "model": model,
        "max_tokens": request.max_tokens,
        "system": request.system,
        "messages": request.messages.iter().map(message).collect::<Vec<_>>(),
        "stream": true,
    });
    if !request.tools.is_empty() {
        body["tools"] = json!(request.tools.iter().map(tool).collect::<Vec<_>>());
        body["tool_choice"] = json!({ "type": "auto" });
    }
    body
}

fn tool(spec: &ToolSpec) -> Value {
    json!({
        "name": spec.name,
        "description": spec.description,
        "input_schema": spec.parameters,
    })
}

fn message(message: &Msg) -> Value {
    match message {
        Msg::User(text) => json!({ "role": "user", "content": text }),
        Msg::Assistant { text, calls } => {
            let mut blocks = Vec::new();
            if !text.is_empty() {
                blocks.push(json!({ "type": "text", "text": text }));
            }
            blocks.extend(calls.iter().map(|call| {
                json!({ "type": "tool_use", "id": call.id, "name": call.name, "input": call.input })
            }));
            json!({ "role": "assistant", "content": blocks })
        }
        Msg::Results(results) => json!({
            "role": "user",
            "content": results.iter().map(|result| json!({
                "type": "tool_result",
                "tool_use_id": result.id,
                "content": result.content,
                "is_error": result.is_error,
            })).collect::<Vec<_>>(),
        }),
    }
}

#[derive(Default)]
struct Pending {
    id: String,
    name: String,
    json: String,
}

#[derive(Default)]
struct Assembly {
    reply: ModelReply,
    blocks: BTreeMap<u64, Pending>,
    failure: Option<CoreError>,
}

impl Assembly {
    async fn on_event<D: DeltaSink>(&mut self, data: &str, deltas: &mut D) -> bool {
        let Ok(event) = serde_json::from_str::<Value>(data) else {
            return true;
        };
        let index = event["index"].as_u64().unwrap_or(0);
        match event["type"].as_str().unwrap_or_default() {
            "message_start" => {
                self.reply.usage.input_tokens = event["message"]["usage"]["input_tokens"]
                    .as_u64()
                    .unwrap_or(0);
                self.reply.usage.output_tokens = event["message"]["usage"]["output_tokens"]
                    .as_u64()
                    .unwrap_or(0);
            }
            "content_block_start" => {
                let block = &event["content_block"];
                if block["type"] == "tool_use" {
                    self.blocks.insert(
                        index,
                        Pending {
                            id: block["id"].as_str().unwrap_or_default().to_owned(),
                            name: block["name"].as_str().unwrap_or_default().to_owned(),
                            json: String::new(),
                        },
                    );
                }
            }
            "content_block_delta" => {
                let delta = &event["delta"];
                match delta["type"].as_str().unwrap_or_default() {
                    "text_delta" => {
                        let text = delta["text"].as_str().unwrap_or_default();
                        self.reply.text.push_str(text);
                        if self.reply.text.len() > MAX_REPLY_BYTES {
                            self.failure = Some(overflow("answer"));
                            return false;
                        }
                        if !text.is_empty() && !deltas.delta(text).await {
                            self.reply.stopped = true;
                            return false;
                        }
                    }
                    "input_json_delta" => {
                        if let Some(pending) = self.blocks.get_mut(&index) {
                            pending
                                .json
                                .push_str(delta["partial_json"].as_str().unwrap_or_default());
                            if pending.json.len() > MAX_TOOL_JSON_BYTES {
                                self.failure = Some(overflow("tool call"));
                                return false;
                            }
                        }
                    }
                    _ => {}
                }
            }
            "message_delta" => {
                if let Some(output) = event["usage"]["output_tokens"].as_u64() {
                    self.reply.usage.output_tokens = output;
                }
            }
            "error" => {
                let message = event["error"]["message"]
                    .as_str()
                    .unwrap_or("unknown error");
                self.failure = Some(CoreError::Internal(format!(
                    "the language model call failed: {message}"
                )));
                return false;
            }
            _ => {}
        }
        true
    }

    fn finish(mut self, model: &str) -> Result<ModelReply, CoreError> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        for pending in self.blocks.into_values() {
            let input = if pending.json.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(&pending.json).unwrap_or(Value::Null)
            };
            self.reply.calls.push(Call {
                id: pending.id,
                name: pending.name,
                input,
            });
        }
        self.reply.usage.model = model.to_owned();
        Ok(self.reply)
    }
}

impl Provider for AnthropicStream {
    async fn stream<D: DeltaSink>(
        &self,
        request: &ModelRequest<'_>,
        deltas: &mut D,
    ) -> Result<ModelReply, CoreError> {
        let mut response = self
            .api
            .messages_request()
            .json(&body(&self.model, request))
            .send()
            .await
            .map_err(|error| {
                CoreError::Internal(format!(
                    "the language model call failed: {}",
                    truncate_chars(&error.without_url().to_string(), ERROR_MAX_CHARS)
                ))
            })?;
        let status = response.status();
        if !status.is_success() {
            let text = read_capped_text(&mut response, MAX_ERROR_BODY_BYTES).await;
            return Err(http_failure(status.as_u16(), &text));
        }
        let mut decoder = SseDecoder::default();
        let mut assembly = Assembly::default();
        'read: loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    for data in decoder.push(&chunk)? {
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
                        truncate_chars(&error.without_url().to_string(), ERROR_MAX_CHARS)
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
    use capitainerie_core::domain::conversation::turn::ToolSpec;

    use super::*;
    use crate::captain::driver::ToolResult;

    struct Collect(String, bool);

    impl DeltaSink for Collect {
        async fn delta(&mut self, text: &str) -> bool {
            self.0.push_str(text);
            self.1
        }
    }

    fn data(events: &[Value]) -> Vec<String> {
        events.iter().map(Value::to_string).collect()
    }

    #[tokio::test]
    async fn text_tool_input_and_usage_are_assembled_from_the_stream() {
        let events = data(&[
            json!({"type": "message_start", "message": {"usage": {"input_tokens": 25, "output_tokens": 1}}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Je cherche"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " Acme."}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "toolu_1", "name": "search_cards", "input": {}}}),
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"query\":"}}),
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "\"acme\"}"}}),
            json!({"type": "content_block_stop", "index": 1}),
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 42}}),
            json!({"type": "message_stop"}),
        ]);
        let mut sink = Collect(String::new(), true);
        let mut assembly = Assembly::default();
        for event in &events {
            assert!(assembly.on_event(event, &mut sink).await);
        }

        let reply = assembly.finish("claude-sonnet-5-5").unwrap();

        assert_eq!(sink.0, "Je cherche Acme.");
        assert_eq!(reply.text, "Je cherche Acme.");
        assert_eq!(
            reply.calls,
            [Call {
                id: "toolu_1".into(),
                name: "search_cards".into(),
                input: json!({"query": "acme"}),
            }]
        );
        assert_eq!(
            (reply.usage.input_tokens, reply.usage.output_tokens),
            (25, 42)
        );
        assert_eq!(reply.usage.model, "claude-sonnet-5-5");
    }

    #[tokio::test]
    async fn a_gone_listener_stops_the_assembly_and_marks_the_reply_stopped() {
        let mut sink = Collect(String::new(), false);
        let mut assembly = Assembly::default();
        let keep_going = assembly
            .on_event(
                &json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Bon"}}).to_string(),
                &mut sink,
            )
            .await;
        assert!(!keep_going);
        let reply = assembly.finish("m").unwrap();
        assert!(reply.stopped);
        assert_eq!(reply.text, "Bon");
    }

    #[tokio::test]
    async fn an_error_event_fails_the_call_with_its_message() {
        let mut sink = Collect(String::new(), true);
        let mut assembly = Assembly::default();
        assembly
            .on_event(
                &json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}).to_string(),
                &mut sink,
            )
            .await;
        let error = assembly.finish("m").unwrap_err();
        assert!(matches!(error, CoreError::Internal(message) if message.contains("Overloaded")));
    }

    #[test]
    fn the_request_carries_tool_blocks_results_and_no_sampling_parameters() {
        let messages = [
            Msg::User("Bonjour".into()),
            Msg::Assistant {
                text: String::new(),
                calls: vec![Call {
                    id: "t1".into(),
                    name: "search_cards".into(),
                    input: json!({"query": "a"}),
                }],
            },
            Msg::Results(vec![ToolResult {
                id: "t1".into(),
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
            "claude-sonnet-5-5",
            &ModelRequest {
                system: "s",
                messages: &messages,
                tools: &tools,
                max_tokens: 100,
            },
        );
        assert_eq!(body["stream"], true);
        assert_eq!(body["tool_choice"], json!({"type": "auto"}));
        assert_eq!(body["messages"][1]["content"][0]["type"], "tool_use");
        assert_eq!(body["messages"][2]["content"][0]["tool_use_id"], "t1");
        assert_eq!(body["tools"][0]["input_schema"], json!({"type": "object"}));
        for forbidden in ["temperature", "top_p", "top_k", "thinking"] {
            assert!(body.get(forbidden).is_none(), "{forbidden}");
        }
        let without = super::body(
            "m",
            &ModelRequest {
                system: "s",
                messages: &messages[..1],
                tools: &[],
                max_tokens: 1,
            },
        );
        assert!(without.get("tools").is_none() && without.get("tool_choice").is_none());
    }

    #[tokio::test]
    async fn an_answer_past_512_kib_fails_the_turn() {
        let mut sink = Collect(String::new(), true);
        let mut assembly = Assembly::default();
        let chunk = "x".repeat(64 * 1024);
        let event = json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": chunk}}).to_string();
        let mut stopped_at = None;
        for n in 0..10 {
            if !assembly.on_event(&event, &mut sink).await {
                stopped_at = Some(n);
                break;
            }
        }

        assert_eq!(stopped_at, Some(8));
        let error = assembly.finish("m").unwrap_err();
        assert!(error.to_string().contains("answer is too large"));
    }

    #[tokio::test]
    async fn tool_input_past_256_kib_fails_the_turn() {
        let mut sink = Collect(String::new(), true);
        let mut assembly = Assembly::default();
        assembly
            .on_event(
                &json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "t", "name": "n", "input": {}}}).to_string(),
                &mut sink,
            )
            .await;
        let piece = json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "x".repeat(64 * 1024)}}).to_string();
        let mut stopped_at = None;
        for n in 0..10 {
            if !assembly.on_event(&piece, &mut sink).await {
                stopped_at = Some(n);
                break;
            }
        }

        assert_eq!(stopped_at, Some(4));
        let error = assembly.finish("m").unwrap_err();
        assert!(error.to_string().contains("tool call is too large"));
    }
}
