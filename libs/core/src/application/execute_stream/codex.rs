use serde_json::Value;

use crate::domain::execution::harness::{AgentEvent, AgentFinish, AgentUsage};

#[derive(Debug, Default)]
pub(super) struct CodexParser {
    usage: AgentUsage,
    summary: Option<String>,
    turns: u64,
}

impl CodexParser {
    pub(super) fn events(&mut self, line: &Value) -> Vec<AgentEvent> {
        match line["type"].as_str() {
            Some("turn.started") => {
                self.turns += 1;
                Vec::new()
            }
            Some("turn.completed") => {
                self.add_usage(&line["usage"]);
                vec![AgentEvent::Finished(self.finish(None))]
            }
            Some("turn.failed") => {
                let reason = line["error"]["message"].as_str().unwrap_or("turn.failed");
                vec![AgentEvent::Finished(self.finish(Some(reason.to_owned())))]
            }
            Some("item.completed") => self.item(&line["item"]),
            _ => Vec::new(),
        }
    }

    fn add_usage(&mut self, usage: &Value) {
        let count = |field: &str| usage[field].as_u64().unwrap_or(0);
        self.usage = AgentUsage {
            input_tokens: self
                .usage
                .input_tokens
                .saturating_add(count("input_tokens")),
            output_tokens: self
                .usage
                .output_tokens
                .saturating_add(count("output_tokens")),
            cache_read_tokens: self
                .usage
                .cache_read_tokens
                .saturating_add(count("cached_input_tokens")),
            cache_creation_tokens: 0,
        };
    }

    fn finish(&self, stopped: Option<String>) -> AgentFinish {
        AgentFinish {
            usage: self.usage,
            summary: self.summary.clone(),
            stopped,
            reported_cost_usd: None,
            turns: Some(self.turns),
        }
    }

    fn item(&mut self, item: &Value) -> Vec<AgentEvent> {
        let Some(id) = item["id"].as_str() else {
            return Vec::new();
        };
        let completed = item["status"] == "completed";
        match item["type"].as_str() {
            Some("agent_message") => {
                self.summary = item["text"].as_str().map(str::to_owned);
                Vec::new()
            }
            Some("command_execution") => call(
                id,
                "shell",
                item["command"].as_str().unwrap_or_default().to_owned(),
                item["aggregated_output"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                completed && item["exit_code"].as_i64() == Some(0),
            ),
            Some("file_change") => call(
                id,
                "file_change",
                changes(&item["changes"]),
                String::new(),
                completed,
            ),
            Some("mcp_tool_call") => {
                let tool = format!(
                    "mcp:{}.{}",
                    item["server"].as_str().unwrap_or("unknown"),
                    item["tool"].as_str().unwrap_or("unknown")
                );
                let failed = !item["error"].is_null();
                let output = if failed {
                    item["error"]["message"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned()
                } else {
                    item["result"]["content"].to_string()
                };
                call(
                    id,
                    &tool,
                    item["arguments"].to_string(),
                    output,
                    completed && !failed,
                )
            }
            Some("web_search") => call(
                id,
                "web_search",
                item["query"].as_str().unwrap_or_default().to_owned(),
                String::new(),
                true,
            ),
            _ => Vec::new(),
        }
    }
}

fn call(id: &str, tool: &str, input: String, output: String, succeeded: bool) -> Vec<AgentEvent> {
    vec![
        AgentEvent::ToolCall {
            id: id.to_owned(),
            tool: tool.to_owned(),
            input,
        },
        AgentEvent::ToolResult {
            id: id.to_owned(),
            output,
            succeeded,
        },
    ]
}

fn changes(changes: &Value) -> String {
    changes
        .as_array()
        .into_iter()
        .flatten()
        .map(|change| {
            format!(
                "{} {}",
                change["kind"].as_str().unwrap_or("update"),
                change["path"].as_str().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    fn completed(item: Value) -> Value {
        json!({"type":"item.completed","item":item})
    }

    #[test]
    fn a_command_gives_a_shell_call_and_its_result() {
        let mut parser = CodexParser::default();
        let ok = completed(
            json!({"id":"item_1","type":"command_execution","command":"ls","aggregated_output":"a\n","exit_code":0,"status":"completed"}),
        );
        let bad = completed(
            json!({"id":"item_2","type":"command_execution","command":"x","aggregated_output":"no","exit_code":2,"status":"failed"}),
        );
        let running = completed(
            json!({"id":"item_3","type":"command_execution","command":"y","aggregated_output":"","exit_code":null,"status":"completed"}),
        );
        assert_eq!(
            parser.events(&ok),
            [
                AgentEvent::ToolCall {
                    id: "item_1".into(),
                    tool: "shell".into(),
                    input: "ls".into()
                },
                AgentEvent::ToolResult {
                    id: "item_1".into(),
                    output: "a\n".into(),
                    succeeded: true
                },
            ]
        );
        assert!(matches!(
            parser.events(&bad).as_slice(),
            [
                _,
                AgentEvent::ToolResult {
                    succeeded: false,
                    ..
                }
            ]
        ));
        assert!(matches!(
            parser.events(&running).as_slice(),
            [
                _,
                AgentEvent::ToolResult {
                    succeeded: false,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn only_a_completed_item_gives_events() {
        let mut parser = CodexParser::default();
        for kind in ["item.started", "item.updated"] {
            let line = json!({"type":kind,"item":{"id":"item_1","type":"command_execution","command":"ls","status":"in_progress"}});
            assert!(parser.events(&line).is_empty());
        }
    }

    #[test]
    fn a_file_change_lists_its_paths() {
        let mut parser = CodexParser::default();
        let line = completed(
            json!({"id":"item_4","type":"file_change","status":"completed","changes":[{"path":"a.md","kind":"add"},{"path":"b.rs","kind":"update"}]}),
        );
        assert_eq!(
            parser.events(&line),
            [
                AgentEvent::ToolCall {
                    id: "item_4".into(),
                    tool: "file_change".into(),
                    input: "add a.md\nupdate b.rs".into()
                },
                AgentEvent::ToolResult {
                    id: "item_4".into(),
                    output: String::new(),
                    succeeded: true
                },
            ]
        );
        let failed =
            completed(json!({"id":"item_5","type":"file_change","status":"failed","changes":[]}));
        assert!(matches!(
            parser.events(&failed).as_slice(),
            [
                _,
                AgentEvent::ToolResult {
                    succeeded: false,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn an_mcp_call_names_its_server_and_tool() {
        let mut parser = CodexParser::default();
        let ok = completed(
            json!({"id":"item_6","type":"mcp_tool_call","server":"docs","tool":"search","arguments":{"q":"x"},"result":{"content":[{"type":"text","text":"hi"}],"structured_content":null},"error":null,"status":"completed"}),
        );
        assert_eq!(
            parser.events(&ok),
            [
                AgentEvent::ToolCall {
                    id: "item_6".into(),
                    tool: "mcp:docs.search".into(),
                    input: r#"{"q":"x"}"#.into()
                },
                AgentEvent::ToolResult {
                    id: "item_6".into(),
                    output: r#"[{"type":"text","text":"hi"}]"#.into(),
                    succeeded: true
                },
            ]
        );
        let failed = completed(
            json!({"id":"item_7","type":"mcp_tool_call","server":"docs","tool":"search","arguments":{},"result":null,"error":{"message":"boom"},"status":"failed"}),
        );
        assert!(matches!(
            parser.events(&failed).as_slice(),
            [_, AgentEvent::ToolResult { output, succeeded: false, .. }] if output == "boom"
        ));
    }

    #[test]
    fn a_web_search_gives_a_call_that_succeeds() {
        let mut parser = CodexParser::default();
        let line = completed(json!({"id":"item_8","type":"web_search","query":"rust"}));
        assert_eq!(
            parser.events(&line),
            [
                AgentEvent::ToolCall {
                    id: "item_8".into(),
                    tool: "web_search".into(),
                    input: "rust".into()
                },
                AgentEvent::ToolResult {
                    id: "item_8".into(),
                    output: String::new(),
                    succeeded: true
                },
            ]
        );
    }

    #[test]
    fn a_completed_turn_finishes_with_its_usage_and_the_last_message() {
        let mut parser = CodexParser::default();
        assert!(parser.events(&json!({"type":"turn.started"})).is_empty());
        assert!(
            parser
                .events(&completed(
                    json!({"id":"item_0","type":"agent_message","text":"first"})
                ))
                .is_empty()
        );
        parser.events(&completed(
            json!({"id":"item_9","type":"agent_message","text":"done"}),
        ));
        let line = json!({"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":60,"cache_write_input_tokens":0,"output_tokens":7,"reasoning_output_tokens":3}});
        assert_eq!(
            parser.events(&line),
            [AgentEvent::Finished(AgentFinish {
                usage: AgentUsage {
                    input_tokens: 100,
                    output_tokens: 7,
                    cache_read_tokens: 60,
                    cache_creation_tokens: 0
                },
                summary: Some("done".into()),
                stopped: None,
                reported_cost_usd: None,
                turns: Some(1)
            })]
        );
    }

    #[test]
    fn usage_adds_up_over_turns() {
        let mut parser = CodexParser::default();
        let line = json!({"type":"turn.completed","usage":{"input_tokens":10,"cached_input_tokens":4,"output_tokens":1}});
        parser.events(&json!({"type":"turn.started"}));
        parser.events(&line);
        parser.events(&json!({"type":"turn.started"}));
        let events = parser.events(&line);
        let [AgentEvent::Finished(finish)] = events.as_slice() else {
            panic!("a turn is one finish");
        };
        assert_eq!(finish.usage.input_tokens, 20);
        assert_eq!(finish.usage.cache_read_tokens, 8);
        assert_eq!(finish.usage.output_tokens, 2);
        assert_eq!(finish.turns, Some(2));
    }

    #[test]
    fn a_failed_turn_says_why_it_stopped() {
        let mut parser = CodexParser::default();
        parser.events(&json!({"type":"turn.started"}));
        let line = json!({"type":"turn.failed","error":{"message":"rate limited"}});
        let events = parser.events(&line);
        let [AgentEvent::Finished(finish)] = events.as_slice() else {
            panic!("a failure is one finish");
        };
        assert_eq!(finish.stopped.as_deref(), Some("rate limited"));
        assert_eq!(finish.usage, AgentUsage::default());
    }

    #[test]
    fn malformed_and_unknown_lines_give_nothing() {
        let mut parser = CodexParser::default();
        for line in [
            json!({"type":"thread.started","thread_id":"t"}),
            json!({"type":"error","message":"x"}),
            json!({"no":"type"}),
            json!("text"),
            json!(null),
            json!({"type":"item.completed"}),
            json!({"type":"item.completed","item":{"type":"command_execution"}}),
            json!({"type":"item.completed","item":{"id":"i","type":"reasoning","text":"t"}}),
            json!({"type":"item.completed","item":{"id":"i","type":"todo_list","items":[]}}),
        ] {
            assert!(parser.events(&line).is_empty(), "{line}");
        }
    }
}
