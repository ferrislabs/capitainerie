use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::provider::ProviderKind;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Harness {
    #[default]
    ClaudeCode,
}

impl Harness {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
        }
    }

    pub const fn provider(self) -> ProviderKind {
        match self {
            Self::ClaudeCode => ProviderKind::Anthropic,
        }
    }

    pub const fn default_model(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-sonnet-5-5",
        }
    }

    pub fn accepts_model(self, model: &str) -> bool {
        match self {
            Self::ClaudeCode => model.starts_with("claude-"),
        }
    }
}

impl fmt::Display for Harness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    ToolCall {
        id: String,
        tool: String,
        input: String,
    },
    ToolResult {
        id: String,
        output: String,
        succeeded: bool,
    },
    Finished(AgentFinish),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentFinish {
    pub usage: AgentUsage,
    pub summary: Option<String>,
    pub stopped: Option<String>,
    pub reported_cost_usd: Option<f64>,
    pub turns: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgentUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_code_runs_on_anthropic_models_only() {
        let harness = Harness::ClaudeCode;
        assert_eq!(harness.provider(), ProviderKind::Anthropic);
        assert!(harness.accepts_model(harness.default_model()));
        for refused in ["", "gpt-4o", " claude-opus-5", "anthropic/claude"] {
            assert!(!harness.accepts_model(refused), "{refused:?}");
        }
    }

    #[test]
    fn a_harness_is_named_in_kebab_case() {
        assert_eq!(Harness::ClaudeCode.to_string(), "claude-code");
        assert_eq!(
            serde_json::to_string(&Harness::ClaudeCode).ok().as_deref(),
            Some("\"claude-code\"")
        );
    }
}
