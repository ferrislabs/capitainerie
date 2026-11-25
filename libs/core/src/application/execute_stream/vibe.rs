use serde_json::Value;

use crate::domain::execution::harness::{AgentEvent, AgentFinish, AgentUsage};

const SETTLED: [&str; 6] = [
    "completed",
    "failed",
    "cancelled",
    "denied",
    "rejected",
    "skipped",
];

#[derive(Debug, Default)]
pub(super) struct VibeParser {
    summary: Option<String>,
}

impl VibeParser {
    pub(super) fn events(&mut self, line: &Value) -> Vec<AgentEvent> {
        match line["type"].as_str() {
            Some("message") => {
                self.message(line);
                Vec::new()
            }
            Some("effect") => effect(line),
            Some("vibe_export") => vec![AgentEvent::Finished(self.finish(&line["export"]))],
            _ => Vec::new(),
        }
    }

    fn message(&mut self, line: &Value) {
        if line["role"] != "assistant" {
            return;
        }
        let text: String = line["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|part| part["type"] == "text")
            .filter_map(|part| part["text"].as_str())
            .collect();
        if !text.is_empty() {
            self.summary = Some(text);
        }
    }

    fn finish(&self, export: &Value) -> AgentFinish {
        let count = |field: &str| export["usage"][field].as_u64().unwrap_or(0);
        AgentFinish {
            usage: AgentUsage {
                input_tokens: count("input_tokens"),
                output_tokens: count("output_tokens"),
                cache_read_tokens: count("cached_input_tokens"),
                cache_creation_tokens: 0,
            },
            summary: self.summary.clone(),
            stopped: stopped(export),
            reported_cost_usd: None,
            turns: export["steps"].as_u64(),
        }
    }
}

fn stopped(export: &Value) -> Option<String> {
    if export["outcome"] == "finished" {
        return None;
    }
    let text = |field: &str| match &export[field] {
        Value::Null => None,
        Value::String(text) if text.is_empty() => None,
        Value::String(text) => Some(text.clone()),
        Value::Object(error) if error.get("message").is_some_and(Value::is_string) => {
            error["message"].as_str().map(str::to_owned)
        }
        other => Some(other.to_string()),
    };
    Some(
        text("error")
            .or_else(|| text("stop_reason"))
            .or_else(|| text("outcome"))
            .unwrap_or_else(|| "unknown".to_owned()),
    )
}

fn effect(line: &Value) -> Vec<AgentEvent> {
    let (Some(id), Some(status)) = (
        line["toolCallId"].as_str(),
        line["state"]["status"].as_str(),
    ) else {
        return Vec::new();
    };
    if !SETTLED.contains(&status) {
        return Vec::new();
    }
    let tool = line["detail"]["toolName"]
        .as_str()
        .or_else(|| line["title"].as_str())
        .unwrap_or("unknown");
    vec![
        AgentEvent::ToolCall {
            id: id.to_owned(),
            tool: tool.to_owned(),
            input: line["detail"]["input"].to_string(),
        },
        AgentEvent::ToolResult {
            id: id.to_owned(),
            output: line["state"]["outputText"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            succeeded: status == "completed" && line["state"]["display"]["success"] != false,
        },
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    fn tool(status: &str, success: Value) -> Value {
        json!({"type":"effect","title":"file_system.bash","toolCallId":"c1",
            "detail":{"toolName":"file_system.bash","input":{"command":"ls"}},
            "state":{"status":status,"outputText":"a\nb","display":{"success":success}}})
    }

    fn assistant(text: &str) -> Value {
        json!({"type":"message","role":"assistant","content":[{"type":"text","text":text}]})
    }

    #[test]
    fn a_settled_effect_is_a_call_and_its_result() {
        let events = VibeParser::default().events(&tool("completed", json!(true)));
        assert_eq!(
            events,
            [
                AgentEvent::ToolCall {
                    id: "c1".into(),
                    tool: "file_system.bash".into(),
                    input: r#"{"command":"ls"}"#.into()
                },
                AgentEvent::ToolResult {
                    id: "c1".into(),
                    output: "a\nb".into(),
                    succeeded: true
                },
            ]
        );
    }

    #[test]
    fn a_failed_or_unsuccessful_effect_does_not_succeed() {
        let mut parser = VibeParser::default();
        for line in [
            tool("failed", json!(true)),
            tool("cancelled", json!(null)),
            tool("completed", json!(false)),
        ] {
            assert!(matches!(
                parser.events(&line)[1],
                AgentEvent::ToolResult {
                    succeeded: false,
                    ..
                }
            ));
        }
    }

    #[test]
    fn a_completed_effect_without_a_display_succeeds() {
        let mut line = tool("completed", json!(true));
        line["state"]["display"] = Value::Null;
        assert!(matches!(
            VibeParser::default().events(&line)[1],
            AgentEvent::ToolResult {
                succeeded: true,
                ..
            }
        ));
    }

    #[test]
    fn a_running_effect_and_other_lines_give_nothing() {
        let mut parser = VibeParser::default();
        assert!(parser.events(&tool("running", json!(null))).is_empty());
        assert!(
            parser
                .events(&json!({"type":"effect","state":{"status":"completed"}}))
                .is_empty()
        );
        assert!(parser.events(&json!({})).is_empty());
        assert!(parser.events(&json!("text")).is_empty());
        assert!(parser.events(&assistant("hi")).is_empty());
    }

    #[test]
    fn the_export_finishes_with_usage_and_the_last_assistant_text() {
        let mut parser = VibeParser::default();
        parser.events(&assistant("first"));
        parser.events(
            &json!({"type":"message","role":"user","content":[{"type":"text","text":"x"}]}),
        );
        parser.events(&assistant(""));
        parser.events(&assistant("last"));
        let export = json!({"type":"vibe_export","export":{"outcome":"finished","stop_reason":null,"error":null,
            "usage":{"input_tokens":2000,"output_tokens":100,"cached_input_tokens":800},"steps":2,"cost_usd":0.5}});
        assert_eq!(
            parser.events(&export),
            [AgentEvent::Finished(AgentFinish {
                usage: AgentUsage {
                    input_tokens: 2_000,
                    output_tokens: 100,
                    cache_read_tokens: 800,
                    cache_creation_tokens: 0
                },
                summary: Some("last".into()),
                stopped: None,
                reported_cost_usd: None,
                turns: Some(2)
            })]
        );
    }

    #[test]
    fn an_export_that_did_not_finish_says_why() {
        let finish = |export: Value| match VibeParser::default()
            .events(&json!({"type":"vibe_export","export":export}))
            .as_slice()
        {
            [AgentEvent::Finished(finish)] => finish.stopped.clone(),
            other => panic!("an export is one finish, got {other:?}"),
        };
        assert_eq!(
            finish(json!({"outcome":"failed","error":"quota","stop_reason":"x"})).as_deref(),
            Some("quota")
        );
        assert_eq!(
            finish(json!({"outcome":"stopped","error":null,"stop_reason":"iteration_limit"}))
                .as_deref(),
            Some("iteration_limit")
        );
        assert_eq!(
            finish(json!({"outcome":"aborted","error":""})).as_deref(),
            Some("aborted")
        );
        assert_eq!(
            finish(json!({"outcome":"infrastructure_failure","error":{"message":"spawn failed"}}))
                .as_deref(),
            Some("spawn failed")
        );
        assert_eq!(finish(json!({})).as_deref(), Some("unknown"));
    }
}
