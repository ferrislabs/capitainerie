use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::provider::ProviderKind;

pub const CODEX_MODELS: [&str; 9] = [
    "gpt-6.1-sol",
    "gpt-6.1-terra",
    "gpt-6-astra",
    "gpt-6-sol",
    "gpt-6-luna",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-5.5",
];

pub const ANTIGRAVITY_MODELS: [(&str, &str); 4] = [
    ("gemini-4-argon", "gemini-4-argon"),
    ("gemini-3.8-flash", "gemini-3.8-flash"),
    ("gemini-3.6-flash", "gemini-3.6-flash"),
    ("gemini-3.1-pro", "gemini-3.1-pro-preview"),
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Harness {
    #[default]
    ClaudeCode,
    Codex,
    Antigravity,
}

impl Harness {
    pub const ALL: [Harness; 3] = [Self::ClaudeCode, Self::Codex, Self::Antigravity];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Antigravity => "antigravity",
        }
    }

    pub const fn provider(self) -> ProviderKind {
        match self {
            Self::ClaudeCode => ProviderKind::Anthropic,
            Self::Codex => ProviderKind::OpenAi,
            Self::Antigravity => ProviderKind::Google,
        }
    }

    pub const fn default_model(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-sonnet-5-5",
            Self::Codex => CODEX_MODELS[0],
            Self::Antigravity => ANTIGRAVITY_MODELS[0].0,
        }
    }

    pub fn accepts_model(self, model: &str) -> bool {
        match self {
            Self::ClaudeCode => model.starts_with("claude-"),
            Self::Codex => CODEX_MODELS.contains(&model),
            Self::Antigravity => ANTIGRAVITY_MODELS.iter().any(|(id, _)| *id == model),
        }
    }

    pub fn wire_model(self, model: &str) -> &str {
        match self {
            Self::ClaudeCode | Self::Codex => model,
            Self::Antigravity => ANTIGRAVITY_MODELS
                .iter()
                .find(|(id, _)| *id == model)
                .map_or(model, |(_, wire)| wire),
        }
    }

    pub fn of_model(model: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|harness| harness.accepts_model(model))
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

    #[test]
    fn codex_runs_on_openai_models_only() {
        let harness = Harness::Codex;
        assert_eq!(harness.provider(), ProviderKind::OpenAi);
        assert!(harness.accepts_model(harness.default_model()));
        for refused in ["claude-sonnet-5-5", "gpt-4o", ""] {
            assert!(!harness.accepts_model(refused), "{refused:?}");
        }
    }

    #[test]
    fn antigravity_runs_on_google_cli_model_ids_only() {
        let harness = Harness::Antigravity;
        assert_eq!(harness.provider(), ProviderKind::Google);
        assert!(harness.accepts_model(harness.default_model()));
        for (id, _) in ANTIGRAVITY_MODELS {
            assert!(harness.accepts_model(id), "{id}");
        }
        for refused in [
            "gemini-3.1-pro-preview",
            "gemini-3.7-flash",
            "claude-sonnet-5-5",
            "",
        ] {
            assert!(!harness.accepts_model(refused), "{refused:?}");
        }
    }

    #[test]
    fn only_antigravity_translates_a_model_to_its_wire_id() {
        assert_eq!(
            Harness::Antigravity.wire_model("gemini-3.1-pro"),
            "gemini-3.1-pro-preview"
        );
        assert_eq!(
            Harness::Antigravity.wire_model("gemini-3.8-flash"),
            "gemini-3.8-flash"
        );
        assert_eq!(Harness::Antigravity.wire_model("other"), "other");
        assert_eq!(
            Harness::Codex.wire_model("gemini-3.1-pro"),
            "gemini-3.1-pro"
        );
        assert_eq!(
            Harness::ClaudeCode.wire_model("claude-sonnet-5-5"),
            "claude-sonnet-5-5"
        );
    }

    #[test]
    fn a_model_finds_its_harness() {
        assert_eq!(
            Harness::of_model("claude-opus-5-5"),
            Some(Harness::ClaudeCode)
        );
        assert_eq!(Harness::of_model("gpt-6-luna"), Some(Harness::Codex));
        assert_eq!(Harness::of_model("mistral-large"), None);
    }

    #[test]
    fn exactly_one_harness_accepts_each_known_model() {
        let models = CODEX_MODELS
            .into_iter()
            .chain(ANTIGRAVITY_MODELS.map(|(id, _)| id))
            .chain(Harness::ALL.map(Harness::default_model));
        for model in models {
            let accepting = Harness::ALL
                .into_iter()
                .filter(|harness| harness.accepts_model(model))
                .count();
            assert_eq!(accepting, 1, "{model}");
        }
    }
}
