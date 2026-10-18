use std::{collections::HashMap, time::Instant};

use serde_json::Value;

use crate::domain::{
    agent::ToolCall,
    execution::{TaskLine, TaskMarker},
};

pub(super) const REDACTED: &str = "***";
const INPUT_MAX_CHARS: usize = 200;
const OUTPUT_MAX_CHARS: usize = 300;
const SUMMARY_MAX_CHARS: usize = 4_000;

#[derive(Debug, Clone)]
pub(super) struct Redactor {
    secret: String,
}

impl Redactor {
    pub(super) fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_owned(),
        }
    }

    pub(super) fn apply(&self, text: &str) -> String {
        if self.secret.is_empty() {
            return text.to_owned();
        }
        text.replace(&self.secret, REDACTED)
    }
}

pub(super) fn truncate_chars(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((index, _)) => text[..index].to_owned(),
        None => text.to_owned(),
    }
}

struct PendingCall {
    tool: String,
    input: String,
    since: Instant,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Digest {
    pub tool_calls: Vec<ToolCall>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub summary: Option<String>,
    pub stopped: Option<String>,
    pub marker: Option<TaskMarker>,
    pub reported_cost_usd: Option<f64>,
    pub turns: Option<u64>,
}

pub(super) struct StreamReader {
    redactor: Redactor,
    digest: Digest,
    pending: HashMap<String, PendingCall>,
    order: Vec<String>,
    dirty: bool,
}

impl StreamReader {
    pub(super) fn new(redactor: Redactor) -> Self {
        Self {
            redactor,
            digest: Digest::default(),
            pending: HashMap::new(),
            order: Vec::new(),
            dirty: false,
        }
    }

    pub(super) fn reset(&mut self) {
        self.digest = Digest::default();
        self.pending.clear();
        self.order.clear();
        self.dirty = true;
    }

    pub(super) fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub(super) fn snapshot(&self) -> &Digest {
        &self.digest
    }

    pub(super) fn marker(&self) -> Option<&TaskMarker> {
        self.digest.marker.as_ref()
    }

    pub(super) fn accept(&mut self, line: &TaskLine) {
        match line {
            TaskLine::Capitainerie(marker) => {
                self.digest.marker = Some(marker.clone());
                self.dirty = true;
            }
            TaskLine::Agent(event) => self.accept_event(event),
        }
    }

    fn accept_event(&mut self, event: &Value) {
        match event["type"].as_str() {
            Some("assistant") => self.tool_uses(&event["message"]["content"]),
            Some("user") => self.tool_results(&event["message"]["content"]),
            Some("result") => self.result(event),
            _ => {}
        }
    }

    fn tool_uses(&mut self, content: &Value) {
        for item in content.as_array().into_iter().flatten() {
            if item["type"] != "tool_use" {
                continue;
            }
            let Some(id) = item["id"].as_str() else {
                continue;
            };
            let input = self.redactor.apply(&item["input"].to_string());
            self.order.push(id.to_owned());
            self.pending.insert(
                id.to_owned(),
                PendingCall {
                    tool: item["name"].as_str().unwrap_or("unknown").to_owned(),
                    input,
                    since: Instant::now(),
                },
            );
        }
    }

    fn tool_results(&mut self, content: &Value) {
        for item in content.as_array().into_iter().flatten() {
            if item["type"] != "tool_result" {
                continue;
            }
            let Some(call) = item["tool_use_id"]
                .as_str()
                .and_then(|id| self.pending.remove(id))
            else {
                continue;
            };
            let output = self.redactor.apply(&result_text(&item["content"]));
            let succeeded = !item["is_error"].as_bool().unwrap_or(false);
            self.digest.tool_calls.push(ToolCall {
                tool: call.tool,
                succeeded,
                latency_ms: u64::try_from(call.since.elapsed().as_millis()).unwrap_or(u64::MAX),
                summary: call_summary(&call.input, &output),
            });
            self.dirty = true;
        }
    }

    fn result(&mut self, event: &Value) {
        let usage = &event["usage"];
        let count = |field: &str| usage[field].as_u64().unwrap_or(0);
        self.digest.input_tokens = count("input_tokens")
            .saturating_add(count("cache_read_input_tokens"))
            .saturating_add(count("cache_creation_input_tokens"));
        self.digest.output_tokens = count("output_tokens");
        self.digest.summary = event["result"]
            .as_str()
            .map(|text| truncate_chars(&self.redactor.apply(text), SUMMARY_MAX_CHARS))
            .filter(|text| !text.trim().is_empty());
        let subtype = event["subtype"].as_str().unwrap_or("success");
        self.digest.stopped =
            (subtype != "success" || event["is_error"] == true).then(|| subtype.to_owned());
        self.digest.reported_cost_usd = event["total_cost_usd"].as_f64();
        self.digest.turns = event["num_turns"].as_u64();
        self.dirty = true;
    }

    pub(super) fn finish(mut self) -> Digest {
        for id in std::mem::take(&mut self.order) {
            if let Some(call) = self.pending.remove(&id) {
                self.digest.tool_calls.push(ToolCall {
                    tool: call.tool,
                    succeeded: false,
                    latency_ms: u64::try_from(call.since.elapsed().as_millis()).unwrap_or(u64::MAX),
                    summary: call_summary(&call.input, "(no result)"),
                });
            }
        }
        self.digest
    }
}

fn call_summary(input: &str, output: &str) -> String {
    format!(
        "{}\n→ {}",
        truncate_chars(input, INPUT_MAX_CHARS),
        truncate_chars(output, OUTPUT_MAX_CHARS)
    )
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

    const SAMPLE: &str = include_str!("../../tests/fixtures/claude-stream.jsonl");

    fn lines(sample: &str) -> Vec<TaskLine> {
        sample
            .lines()
            .map(|line| {
                let value: Value = serde_json::from_str(line).unwrap();
                if value["type"] == "capitainerie" {
                    TaskLine::Capitainerie(serde_json::from_value(value).unwrap())
                } else {
                    TaskLine::Agent(value)
                }
            })
            .collect()
    }

    fn read(sample: &str, secret: &str) -> Digest {
        let mut reader = StreamReader::new(Redactor::new(secret));
        for line in lines(sample) {
            reader.accept(&line);
        }
        reader.finish()
    }

    #[test]
    fn a_recorded_stream_gives_tool_calls_usage_summary_and_marker() {
        let digest = read(SAMPLE, "");

        let tools: Vec<_> = digest
            .tool_calls
            .iter()
            .map(|call| (call.tool.as_str(), call.succeeded))
            .collect();
        assert_eq!(
            tools,
            [("Bash", true), ("Write", true), ("Bash", false)],
            "{digest:?}"
        );
        assert!(digest.tool_calls[0].summary.contains("ls -la"));
        assert!(digest.tool_calls[0].summary.contains("README.md"));
        assert!(digest.tool_calls[1].summary.contains("CHANGELOG.md"));
        assert!(digest.tool_calls[2].summary.contains("jest: not found"));
        assert_eq!(digest.input_tokens, 9 + 16_360 + 5_805);
        assert_eq!(digest.output_tokens, 283);
        assert!(
            digest
                .summary
                .as_deref()
                .unwrap()
                .starts_with("I added CHANGELOG.md")
        );
        assert_eq!(digest.stopped, None);
        assert_eq!(digest.reported_cost_usd, Some(0.0291));
        assert_eq!(digest.turns, Some(4));
        assert_eq!(
            digest.marker,
            Some(TaskMarker::Pushed {
                sha: "3f9c2a7d1e5b48a6c0d7e9f1a2b3c4d5e6f70812".into()
            })
        );
    }

    #[test]
    fn the_task_token_never_survives_in_a_stored_field() {
        let secret = "dG9rZW4tZGUtdGFjaGUtMDEyMzQ1Njc4OWFiY2RlZg";
        let sample = format!(
            "{}\n{}\n{}\n{}",
            json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":format!("echo ${{CAPITAINERIE_TASK_TOKEN}} {secret}")}}]}}),
            json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":format!("token={secret}"),"is_error":false}]}}),
            json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":"t2","name":"Read","input":{"file_path":secret}}]}}),
            json!({"type":"result","subtype":"success","result":format!("I saw {secret} in the env"),"usage":{"input_tokens":1,"output_tokens":1}}),
        );

        let digest = read(&sample, secret);

        let stored = format!("{digest:?}");
        assert!(!stored.contains(secret), "{stored}");
        assert!(stored.contains(REDACTED));
        assert_eq!(digest.tool_calls.len(), 2);
        assert!(digest.summary.unwrap().contains("I saw *** in the env"));
    }

    #[test]
    fn a_call_without_result_is_kept_as_failed() {
        let sample = json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"sleep 999"}}]}}).to_string();
        let digest = read(&sample, "");
        assert_eq!(digest.tool_calls.len(), 1);
        assert!(!digest.tool_calls[0].succeeded);
        assert!(digest.tool_calls[0].summary.contains("(no result)"));
    }

    #[test]
    fn outputs_are_cut() {
        let long = "x".repeat(5_000);
        let sample = format!(
            "{}\n{}",
            json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":long}}]}}),
            json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":long}]}}),
        );
        let digest = read(&sample, "");
        let summary = &digest.tool_calls[0].summary;
        assert!(summary.chars().count() <= INPUT_MAX_CHARS + OUTPUT_MAX_CHARS + 3);
    }

    #[test]
    fn a_result_that_is_not_a_success_says_why() {
        let sample = json!({"type":"result","subtype":"error_max_turns","is_error":false,"usage":{"input_tokens":4,"output_tokens":2}}).to_string();
        let digest = read(&sample, "");
        assert_eq!(digest.stopped.as_deref(), Some("error_max_turns"));
        assert_eq!(digest.summary, None);
    }

    #[test]
    fn a_reattached_follower_starts_from_a_clean_slate() {
        let mut reader = StreamReader::new(Redactor::new(""));
        for line in lines(SAMPLE) {
            reader.accept(&line);
        }
        reader.reset();
        assert!(reader.snapshot().tool_calls.is_empty());
        assert_eq!(reader.marker(), None);
        for line in lines(SAMPLE) {
            reader.accept(&line);
        }
        assert_eq!(reader.finish().tool_calls.len(), 3);
    }

    #[test]
    fn an_empty_secret_redacts_nothing() {
        assert_eq!(Redactor::new("").apply("abc"), "abc");
    }
}
