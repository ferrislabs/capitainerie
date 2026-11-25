mod antigravity;
mod claude_code;
mod codex;
mod vibe;

use std::{collections::HashMap, time::Instant};

use serde_json::Value;

use crate::domain::{
    agent::ToolCall,
    execution::{
        TaskLine, TaskMarker,
        harness::{AgentEvent, AgentFinish, Harness},
        report::{ReportAssembler, ReportOutcome},
    },
    run::step::CachedUsage,
};

pub(super) const REDACTED: &str = "***";
const INPUT_MAX_CHARS: usize = 200;
const OUTPUT_MAX_CHARS: usize = 300;
const SUMMARY_MAX_CHARS: usize = 4_000;
pub(super) const TOOL_CALLS_MAX: usize = 500;

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
    pub dropped_tool_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub summary: Option<String>,
    pub stopped: Option<String>,
    pub marker: Option<TaskMarker>,
    pub reported_cost_usd: Option<f64>,
    pub turns: Option<u64>,
}

impl Digest {
    pub(super) fn cached_usage(&self) -> CachedUsage {
        CachedUsage {
            fresh_input: self
                .input_tokens
                .saturating_sub(self.cache_read_tokens)
                .saturating_sub(self.cache_creation_tokens),
            cache_read: self.cache_read_tokens,
            cache_creation: self.cache_creation_tokens,
            output: self.output_tokens,
        }
    }

    /// The cost the harness reports, for a model without a known price.
    pub(super) fn reported_cost_micros(&self) -> i64 {
        self.reported_cost_usd
            .filter(|usd| usd.is_finite() && *usd > 0.0)
            .map_or(0, |usd| (usd * 1_000_000.0).round() as i64)
    }

    fn keep(&mut self, call: ToolCall) {
        if self.tool_calls.len() < TOOL_CALLS_MAX {
            self.tool_calls.push(call);
        } else {
            self.dropped_tool_calls += 1;
        }
    }
}

pub(super) struct StreamReader {
    harness: Harness,
    parser: AgentParser,
    redactor: Redactor,
    digest: Digest,
    report: ReportAssembler,
    pending: HashMap<String, PendingCall>,
    order: Vec<String>,
    dirty: bool,
}

impl StreamReader {
    pub(super) fn new(harness: Harness, redactor: Redactor) -> Self {
        Self {
            harness,
            parser: AgentParser::new(harness),
            redactor,
            digest: Digest::default(),
            report: ReportAssembler::default(),
            pending: HashMap::new(),
            order: Vec::new(),
            dirty: false,
        }
    }

    pub(super) fn reset(&mut self) {
        self.parser = AgentParser::new(self.harness);
        self.digest = Digest::default();
        self.report = ReportAssembler::default();
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

    /// The audit report the output carried, redacted.
    pub(super) fn report(&self) -> ReportOutcome {
        match self.report.outcome() {
            ReportOutcome::Ready { text, truncated } => ReportOutcome::Ready {
                text: self.redactor.apply(&text),
                truncated,
            },
            other => other,
        }
    }

    pub(super) fn accept(&mut self, line: &TaskLine) {
        match line {
            TaskLine::Capitainerie(TaskMarker::ReportChunk { data }) => self.report.chunk(data),
            TaskLine::Capitainerie(TaskMarker::ReportEnd {
                sha256,
                bytes,
                truncated,
            }) => {
                self.report.end(sha256, *bytes, *truncated);
                self.dirty = true;
            }
            TaskLine::Capitainerie(marker) => {
                self.digest.marker = Some(marker.clone());
                self.dirty = true;
            }
            TaskLine::Agent(line) => {
                for event in self.parser.events(line) {
                    self.apply(event);
                }
            }
        }
    }

    fn apply(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::ToolCall { id, tool, input } => {
                let input = self.redactor.apply(&input);
                self.order.push(id.clone());
                self.pending.insert(
                    id,
                    PendingCall {
                        tool,
                        input,
                        since: Instant::now(),
                    },
                );
            }
            AgentEvent::ToolResult {
                id,
                output,
                succeeded,
            } => {
                let Some(call) = self.pending.remove(&id) else {
                    return;
                };
                let output = self.redactor.apply(&output);
                self.digest.keep(ToolCall {
                    tool: call.tool,
                    succeeded,
                    latency_ms: u64::try_from(call.since.elapsed().as_millis()).unwrap_or(u64::MAX),
                    summary: call_summary(&call.input, &output),
                });
                self.dirty = true;
            }
            AgentEvent::Finished(finish) => self.finished(finish),
        }
    }

    fn finished(&mut self, finish: AgentFinish) {
        self.digest.input_tokens = finish.usage.input_tokens;
        self.digest.output_tokens = finish.usage.output_tokens;
        self.digest.cache_read_tokens = finish.usage.cache_read_tokens;
        self.digest.cache_creation_tokens = finish.usage.cache_creation_tokens;
        self.digest.summary = finish
            .summary
            .map(|text| truncate_chars(&self.redactor.apply(&text), SUMMARY_MAX_CHARS))
            .filter(|text| !text.trim().is_empty());
        self.digest.stopped = finish.stopped;
        self.digest.reported_cost_usd = finish.reported_cost_usd;
        self.digest.turns = finish.turns;
        self.dirty = true;
    }

    pub(super) fn finish(mut self) -> Digest {
        for id in std::mem::take(&mut self.order) {
            if let Some(call) = self.pending.remove(&id) {
                self.digest.keep(ToolCall {
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

enum AgentParser {
    ClaudeCode,
    Antigravity,
    Codex(codex::CodexParser),
    MistralVibe(vibe::VibeParser),
}

impl AgentParser {
    fn new(harness: Harness) -> Self {
        match harness {
            Harness::ClaudeCode => Self::ClaudeCode,
            Harness::Codex => Self::Codex(codex::CodexParser::default()),
            Harness::MistralVibe => Self::MistralVibe(vibe::VibeParser::default()),
            Harness::Antigravity => Self::Antigravity,
        }
    }

    fn events(&mut self, line: &Value) -> Vec<AgentEvent> {
        match self {
            Self::ClaudeCode => claude_code::events(line),
            Self::Antigravity => antigravity::events(line),
            Self::Codex(parser) => parser.events(line),
            Self::MistralVibe(parser) => parser.events(line),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    const SAMPLE: &str = include_str!("../../../tests/fixtures/claude-stream.jsonl");

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
        let mut reader = StreamReader::new(Harness::ClaudeCode, Redactor::new(secret));
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
        assert_eq!(digest.cache_read_tokens, 16_360);
        assert_eq!(digest.cache_creation_tokens, 5_805);
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

    const CODEX_SAMPLE: &str = include_str!("../../../tests/fixtures/codex-stream.jsonl");

    #[test]
    fn a_reattached_follower_does_not_count_a_codex_turn_twice() {
        let mut reader = StreamReader::new(Harness::Codex, Redactor::new(""));
        for line in lines(CODEX_SAMPLE) {
            reader.accept(&line);
        }
        reader.reset();
        for line in lines(CODEX_SAMPLE) {
            reader.accept(&line);
        }
        let digest = reader.finish();
        assert_eq!(digest.input_tokens, 27_500);
        assert_eq!(digest.output_tokens, 420);
        assert_eq!(digest.tool_calls.len(), 3, "{digest:?}");
    }

    #[test]
    fn a_recorded_codex_stream_gives_tool_calls_usage_summary_and_marker() {
        let mut reader = StreamReader::new(Harness::Codex, Redactor::new(""));
        for line in lines(CODEX_SAMPLE) {
            reader.accept(&line);
        }
        let digest = reader.finish();

        let tools: Vec<_> = digest
            .tool_calls
            .iter()
            .map(|call| (call.tool.as_str(), call.succeeded))
            .collect();
        assert_eq!(
            tools,
            [("shell", true), ("file_change", true), ("shell", false)],
            "{digest:?}"
        );
        assert!(digest.tool_calls[0].summary.contains("ls -la"));
        assert!(digest.tool_calls[1].summary.contains("add CHANGELOG.md"));
        assert!(digest.tool_calls[2].summary.contains("jest: not found"));
        assert_eq!(digest.input_tokens, 27_500);
        assert_eq!(digest.cache_read_tokens, 21_000);
        assert_eq!(digest.cache_creation_tokens, 0);
        assert_eq!(digest.output_tokens, 420);
        assert!(
            digest
                .summary
                .as_deref()
                .unwrap()
                .starts_with("I added CHANGELOG.md")
        );
        assert_eq!(digest.stopped, None);
        assert_eq!(digest.reported_cost_usd, None);
        assert_eq!(digest.turns, Some(1));
        assert_eq!(
            digest.marker,
            Some(TaskMarker::Pushed {
                sha: "3f9c2a7d1e5b48a6c0d7e9f1a2b3c4d5e6f70812".into()
            })
        );
        assert_eq!(digest.cached_usage().fresh_input, 6_500);
    }

    const ANTIGRAVITY_SAMPLE: &str =
        include_str!("../../../tests/fixtures/antigravity-stream.jsonl");

    #[test]
    fn a_recorded_antigravity_stream_gives_tool_calls_usage_summary_and_marker() {
        let mut reader = StreamReader::new(Harness::Antigravity, Redactor::new(""));
        for line in lines(ANTIGRAVITY_SAMPLE) {
            reader.accept(&line);
        }
        let digest = reader.finish();

        let tools: Vec<_> = digest
            .tool_calls
            .iter()
            .map(|call| (call.tool.as_str(), call.succeeded))
            .collect();
        assert_eq!(tools, [("run_command", true)], "{digest:?}");
        assert!(digest.tool_calls[0].summary.contains("hello_lab"));
        assert_eq!(digest.input_tokens, 2_400);
        assert_eq!(digest.cache_read_tokens, 1_600);
        assert_eq!(digest.cache_creation_tokens, 0);
        assert_eq!(digest.output_tokens, 70);
        assert_eq!(digest.summary.as_deref(), Some("Done: I ran echo.\n"));
        assert_eq!(digest.stopped, None);
        assert_eq!(digest.reported_cost_usd, None);
        assert_eq!(digest.turns, Some(1));
        assert_eq!(
            digest.marker,
            Some(TaskMarker::Pushed {
                sha: "3f9c2a7d1e5b48a6c0d7e9f1a2b3c4d5e6f70812".into()
            })
        );
        assert_eq!(digest.cached_usage().fresh_input, 800);
    }

    const VIBE_SAMPLE: &str = include_str!("../../../tests/fixtures/vibe-stream.jsonl");

    #[test]
    fn a_recorded_vibe_stream_gives_tool_calls_usage_summary_and_marker() {
        let mut reader = StreamReader::new(Harness::MistralVibe, Redactor::new(""));
        for line in lines(VIBE_SAMPLE) {
            reader.accept(&line);
        }
        let digest = reader.finish();

        let tools: Vec<_> = digest
            .tool_calls
            .iter()
            .map(|call| (call.tool.as_str(), call.succeeded))
            .collect();
        assert_eq!(tools, [("file_system.bash", true)], "{digest:?}");
        assert!(digest.tool_calls[0].summary.contains("VIBE_LAB_OK"));
        assert_eq!(digest.input_tokens, 2_000);
        assert_eq!(digest.cache_read_tokens, 800);
        assert_eq!(digest.cache_creation_tokens, 0);
        assert_eq!(digest.output_tokens, 100);
        assert_eq!(
            digest.summary.as_deref(),
            Some("Done: the command printed VIBE_LAB_OK.")
        );
        assert_eq!(digest.stopped, None);
        assert_eq!(digest.reported_cost_usd, None);
        assert_eq!(digest.turns, Some(2));
        assert_eq!(
            digest.marker,
            Some(TaskMarker::Pushed {
                sha: "3f9c2a7d1e5b48a6c0d7e9f1a2b3c4d5e6f70812".into()
            })
        );
        assert_eq!(digest.cached_usage().fresh_input, 1_200);
    }

    #[test]
    fn a_reattached_follower_does_not_keep_a_vibe_summary_from_the_first_attempt() {
        let mut reader = StreamReader::new(Harness::MistralVibe, Redactor::new(""));
        let sample = lines(VIBE_SAMPLE);
        for line in &sample[..sample.len() - 2] {
            reader.accept(line);
        }
        reader.reset();
        for line in sample
            .iter()
            .filter(|line| matches!(line, TaskLine::Agent(value) if value["type"] == "vibe_export"))
        {
            reader.accept(line);
        }
        assert_eq!(reader.finish().summary, None);
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
    fn tool_calls_beyond_the_cap_are_counted_not_stored() {
        let mut sample = Vec::new();
        for n in 0..(TOOL_CALLS_MAX + 7) {
            sample.push(json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":format!("t{n}"),"name":"Bash","input":{"command":"ls"}}]}}).to_string());
            sample.push(json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":format!("t{n}"),"content":"ok"}]}}).to_string());
        }
        let digest = read(&sample.join("\n"), "");
        assert_eq!(digest.tool_calls.len(), TOOL_CALLS_MAX);
        assert_eq!(digest.dropped_tool_calls, 7);
    }

    #[test]
    fn the_reported_cost_stands_in_for_an_unpriced_model() {
        let digest = Digest {
            reported_cost_usd: Some(0.0291),
            ..Digest::default()
        };
        assert_eq!(digest.reported_cost_micros(), 29_100);
        assert_eq!(Digest::default().reported_cost_micros(), 0);
        let negative = Digest {
            reported_cost_usd: Some(-1.0),
            ..Digest::default()
        };
        assert_eq!(negative.reported_cost_micros(), 0);
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
        let mut reader = StreamReader::new(Harness::ClaudeCode, Redactor::new(""));
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

    fn report_lines(text: &str, chunk: usize, sha: Option<String>) -> Vec<TaskLine> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let encoded = STANDARD.encode(text);
        let mut lines: Vec<TaskLine> = encoded
            .as_bytes()
            .chunks(chunk)
            .map(|piece| {
                TaskLine::Capitainerie(TaskMarker::ReportChunk {
                    data: String::from_utf8(piece.to_vec()).unwrap(),
                })
            })
            .collect();
        let digest = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, text.as_bytes());
        let hex: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
        lines.push(TaskLine::Capitainerie(TaskMarker::ReportEnd {
            sha256: sha.unwrap_or(hex),
            bytes: text.len() as u64,
            truncated: false,
        }));
        lines.push(TaskLine::Capitainerie(TaskMarker::AuditDone));
        lines
    }

    #[test]
    fn report_chunks_rebuild_the_report_and_leave_the_marker_alone() {
        let mut reader = StreamReader::new(Harness::ClaudeCode, Redactor::new("tok-secret"));
        for line in lines(SAMPLE) {
            reader.accept(&line);
        }
        let text = "# Rapport\n\nLe jeton tok-secret traînait dans .env.";
        for line in report_lines(text, 5, None) {
            reader.accept(&line);
        }
        assert_eq!(reader.marker(), Some(&TaskMarker::AuditDone));
        assert_eq!(
            reader.report(),
            ReportOutcome::Ready {
                text: "# Rapport\n\nLe jeton *** traînait dans .env.".into(),
                truncated: false
            }
        );
        assert_eq!(reader.finish().tool_calls.len(), 3);
    }

    #[test]
    fn a_report_whose_digest_does_not_match_is_invalid() {
        let mut reader = StreamReader::new(Harness::ClaudeCode, Redactor::new(""));
        for line in report_lines("rapport", 4, Some("00".repeat(32))) {
            reader.accept(&line);
        }
        assert!(matches!(reader.report(), ReportOutcome::Invalid(_)));
    }

    #[test]
    fn a_reattached_follower_rebuilds_the_report_from_scratch() {
        let mut reader = StreamReader::new(Harness::ClaudeCode, Redactor::new(""));
        let sent = report_lines("rapport", 4, None);
        for line in &sent {
            reader.accept(line);
        }
        reader.reset();
        assert_eq!(reader.report(), ReportOutcome::Missing);
        for line in &sent {
            reader.accept(line);
        }
        assert!(matches!(reader.report(), ReportOutcome::Ready { text, .. } if text == "rapport"));
    }

    #[test]
    fn an_empty_secret_redacts_nothing() {
        assert_eq!(Redactor::new("").apply("abc"), "abc");
    }
}
