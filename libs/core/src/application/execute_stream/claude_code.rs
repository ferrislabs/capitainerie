use serde_json::Value;

use crate::domain::execution::harness::{AgentEvent, AgentFinish, AgentUsage};

pub(super) fn events(line: &Value) -> Vec<AgentEvent> {
    match line["type"].as_str() {
        Some("assistant") => tool_uses(&line["message"]["content"]),
        Some("user") => tool_results(&line["message"]["content"]),
        Some("result") => vec![AgentEvent::Finished(finish(line))],
        _ => Vec::new(),
    }
}

fn items<'a>(content: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter(move |item| item["type"] == kind)
}

fn tool_uses(content: &Value) -> Vec<AgentEvent> {
    items(content, "tool_use")
        .filter_map(|item| {
            Some(AgentEvent::ToolCall {
                id: item["id"].as_str()?.to_owned(),
                tool: item["name"].as_str().unwrap_or("unknown").to_owned(),
                input: item["input"].to_string(),
            })
        })
        .collect()
}

fn tool_results(content: &Value) -> Vec<AgentEvent> {
    items(content, "tool_result")
        .filter_map(|item| {
            Some(AgentEvent::ToolResult {
                id: item["tool_use_id"].as_str()?.to_owned(),
                output: result_text(&item["content"]),
                succeeded: !item["is_error"].as_bool().unwrap_or(false),
            })
        })
        .collect()
}

fn finish(line: &Value) -> AgentFinish {
    let usage = &line["usage"];
    let count = |field: &str| usage[field].as_u64().unwrap_or(0);
    let cache_read_tokens = count("cache_read_input_tokens");
    let cache_creation_tokens = count("cache_creation_input_tokens");
    let subtype = line["subtype"].as_str().unwrap_or("success");
    AgentFinish {
        usage: AgentUsage {
            input_tokens: count("input_tokens")
                .saturating_add(cache_read_tokens)
                .saturating_add(cache_creation_tokens),
            output_tokens: count("output_tokens"),
            cache_read_tokens,
            cache_creation_tokens,
        },
        summary: line["result"].as_str().map(str::to_owned),
        stopped: (subtype != "success" || line["is_error"] == true).then(|| subtype.to_owned()),
        reported_cost_usd: line["total_cost_usd"].as_f64(),
        turns: line["num_turns"].as_u64(),
    }
}

fn result_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    fn finish_of(line: &Value) -> AgentFinish {
        match events(line).as_slice() {
            [AgentEvent::Finished(finish)] => finish.clone(),
            other => panic!("a result is one finish, got {other:?}"),
        }
    }

    #[test]
    fn an_assistant_message_gives_one_call_per_tool_use() {
        let line = json!({"type":"assistant","message":{"content":[
            {"type":"text","text":"Je regarde."},
            {"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}},
            {"type":"tool_use","name":"Read","input":{}},
            {"type":"tool_use","id":"t2","input":{"file_path":"a"}}
        ]}});
        assert_eq!(
            events(&line),
            [
                AgentEvent::ToolCall {
                    id: "t1".into(),
                    tool: "Bash".into(),
                    input: r#"{"command":"ls"}"#.into()
                },
                AgentEvent::ToolCall {
                    id: "t2".into(),
                    tool: "unknown".into(),
                    input: r#"{"file_path":"a"}"#.into()
                },
            ]
        );
    }

    #[test]
    fn a_tool_result_joins_its_text_parts_and_carries_its_error_flag() {
        let line = json!({"type":"user","message":{"content":[
            {"type":"tool_result","tool_use_id":"t1","content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]},
            {"type":"tool_result","tool_use_id":"t2","content":"boom","is_error":true}
        ]}});
        assert_eq!(
            events(&line),
            [
                AgentEvent::ToolResult {
                    id: "t1".into(),
                    output: "a\nb".into(),
                    succeeded: true
                },
                AgentEvent::ToolResult {
                    id: "t2".into(),
                    output: "boom".into(),
                    succeeded: false
                },
            ]
        );
    }

    #[test]
    fn a_result_counts_cached_tokens_as_input() {
        let line = json!({"type":"result","subtype":"success","result":"ok","num_turns":2,"total_cost_usd":0.5,
            "usage":{"input_tokens":3,"output_tokens":4,"cache_read_input_tokens":10,"cache_creation_input_tokens":20}});
        let finish = finish_of(&line);
        assert_eq!(
            finish.usage,
            AgentUsage {
                input_tokens: 33,
                output_tokens: 4,
                cache_read_tokens: 10,
                cache_creation_tokens: 20
            }
        );
        assert_eq!(finish.summary.as_deref(), Some("ok"));
        assert_eq!(finish.stopped, None);
        assert_eq!(finish.turns, Some(2));
        assert_eq!(finish.reported_cost_usd, Some(0.5));
    }

    #[test]
    fn an_error_result_says_why_it_stopped() {
        let max_turns = json!({"type":"result","subtype":"error_max_turns"});
        let flagged = json!({"type":"result","subtype":"success","is_error":true});
        for (line, stopped) in [(max_turns, "error_max_turns"), (flagged, "success")] {
            let finish = finish_of(&line);
            assert_eq!(finish.stopped.as_deref(), Some(stopped));
        }
    }

    #[test]
    fn other_lines_give_nothing() {
        assert!(events(&json!({"type":"system","subtype":"init"})).is_empty());
        assert!(events(&json!({"no":"type"})).is_empty());
    }
}
