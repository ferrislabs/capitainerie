use serde_json::Value;

use crate::domain::execution::harness::{AgentEvent, AgentFinish, AgentUsage};

pub(super) fn events(line: &Value) -> Vec<AgentEvent> {
    match line["event"].as_str() {
        Some("step_update") => tool_step(&line["step_update"]),
        Some("result") => vec![AgentEvent::Finished(finish(&line["result"]))],
        _ => Vec::new(),
    }
}

fn tool_step(step: &Value) -> Vec<AgentEvent> {
    if step["step_type"] != "tool" {
        return Vec::new();
    }
    let succeeded = match step["state"].as_str() {
        Some("DONE") => true,
        Some("ERROR") => false,
        _ => return Vec::new(),
    };
    let Some(index) = step["step_index"].as_u64() else {
        return Vec::new();
    };
    let info = &step["tool_info"];
    let id = index.to_string();
    let output = if succeeded {
        info["output"].as_str()
    } else {
        info["error"]["message"].as_str()
    };
    vec![
        AgentEvent::ToolCall {
            id: id.clone(),
            tool: info["name"]
                .as_str()
                .or_else(|| step["tool_name"].as_str())
                .unwrap_or("unknown")
                .to_owned(),
            input: info["parameters"].to_string(),
        },
        AgentEvent::ToolResult {
            id,
            output: output.unwrap_or_default().to_owned(),
            succeeded,
        },
    ]
}

fn finish(result: &Value) -> AgentFinish {
    let usage = &result["usage"];
    let count = |field: &str| usage[field].as_u64().unwrap_or(0);
    let cache_read_tokens = count("cache_read_tokens");
    let status = result["status"].as_str().unwrap_or("INVALID");
    let stopped = (status != "SUCCESS").then(|| {
        [result["error"].as_str(), result["finished_reason"].as_str()]
            .into_iter()
            .flatten()
            .find(|text| !text.is_empty())
            .unwrap_or(status)
            .to_owned()
    });
    AgentFinish {
        usage: AgentUsage {
            input_tokens: count("input_tokens").saturating_add(cache_read_tokens),
            output_tokens: count("output_tokens"),
            cache_read_tokens,
            cache_creation_tokens: 0,
        },
        summary: result["response"].as_str().map(str::to_owned),
        stopped,
        reported_cost_usd: None,
        turns: result["num_turns"].as_u64(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn step(state: &str, tool_info: Value) -> Value {
        json!({"event":"step_update","step_update":{"step_index":2,"state":state,"step_type":"tool","tool_name":"run_command","tool_info":tool_info}})
    }

    #[test]
    fn a_done_tool_step_gives_a_call_and_its_output() {
        let line = step(
            "DONE",
            json!({"name":"run_command","parameters":{"CommandLine":"ls"},"output":"a\nb"}),
        );
        assert_eq!(
            events(&line),
            [
                AgentEvent::ToolCall {
                    id: "2".into(),
                    tool: "run_command".into(),
                    input: r#"{"CommandLine":"ls"}"#.into()
                },
                AgentEvent::ToolResult {
                    id: "2".into(),
                    output: "a\nb".into(),
                    succeeded: true
                },
            ]
        );
    }

    #[test]
    fn an_errored_tool_step_fails_with_the_error_message() {
        let line = step(
            "ERROR",
            json!({"name":"run_command","parameters":{},"error":{"type":"x","message":"boom"}}),
        );
        assert_eq!(
            events(&line)[1],
            AgentEvent::ToolResult {
                id: "2".into(),
                output: "boom".into(),
                succeeded: false
            }
        );
    }

    #[test]
    fn the_tool_name_falls_back_to_the_step() {
        let line = step("DONE", json!({"parameters":{}}));
        assert!(
            matches!(&events(&line)[0], AgentEvent::ToolCall { tool, .. } if tool == "run_command")
        );
    }

    #[test]
    fn an_active_tool_step_and_other_steps_give_nothing() {
        assert!(events(&step("ACTIVE", json!({"name":"run_command"}))).is_empty());
        let reply = json!({"event":"step_update","step_update":{"step_index":3,"state":"DONE","step_type":"agent_response","text_delta":"hi"}});
        assert!(events(&reply).is_empty());
        assert!(events(&json!({"event":"init"})).is_empty());
    }

    #[test]
    fn a_successful_result_counts_cache_reads_as_input() {
        let line = json!({"event":"result","result":{"status":"SUCCESS","response":"ok","num_turns":1,
            "usage":{"input_tokens":400,"output_tokens":35,"thinking_tokens":5,"cache_read_tokens":800}}});
        assert_eq!(
            events(&line),
            [AgentEvent::Finished(AgentFinish {
                usage: AgentUsage {
                    input_tokens: 1_200,
                    output_tokens: 35,
                    cache_read_tokens: 800,
                    cache_creation_tokens: 0
                },
                summary: Some("ok".into()),
                stopped: None,
                reported_cost_usd: None,
                turns: Some(1)
            })]
        );
    }

    #[test]
    fn a_failed_result_says_why_it_stopped() {
        let with_error = json!({"event":"result","result":{"status":"ERROR","error":"quota exhausted","finished_reason":"error"}});
        let with_reason = json!({"event":"result","result":{"status":"ERROR","error":"","finished_reason":"print_timeout"}});
        let bare = json!({"event":"result","result":{"status":"CANCELED"}});
        for (line, stopped) in [
            (with_error, "quota exhausted"),
            (with_reason, "print_timeout"),
            (bare, "CANCELED"),
        ] {
            match events(&line).as_slice() {
                [AgentEvent::Finished(finish)] => {
                    assert_eq!(finish.stopped.as_deref(), Some(stopped));
                }
                other => panic!("a result is one finish, got {other:?}"),
            }
        }
    }

    #[test]
    fn malformed_lines_give_nothing() {
        assert!(events(&json!({})).is_empty());
        assert!(events(&json!("text")).is_empty());
        assert!(events(&json!({"event":"step_update"})).is_empty());
        assert!(
            events(
                &json!({"event":"step_update","step_update":{"step_type":"tool","state":"DONE"}})
            )
            .is_empty()
        );
    }
}
