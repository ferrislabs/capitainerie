use capitainerie_common::{OrganizationId, RunId, StepId};
use chrono::{DateTime, Utc};

use super::StepKind;
use crate::domain::agent::{ToolCall, Usage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Running,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub id: StepId,
    pub run_id: RunId,
    pub organization_id: OrganizationId,
    /// Position in the pipeline; a run resumes at the first step not succeeded.
    pub seq: i32,
    pub kind: StepKind,
    /// Client id of the agent that ran the step.
    pub agent: Option<String>,
    pub status: StepStatus,
    pub attempt: i32,
    pub output: Option<serde_json::Value>,
    pub usage: Usage,
    pub cost_micros: i64,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// Prices of the configured model, in micro-units of currency per million
/// tokens. Zero for a local model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pricing {
    pub input_per_million_micros: u64,
    pub output_per_million_micros: u64,
}

impl Pricing {
    pub fn cost_micros(&self, usage: &Usage) -> i64 {
        let cost = u128::from(usage.input_tokens) * u128::from(self.input_per_million_micros)
            + u128::from(usage.output_tokens) * u128::from(self.output_per_million_micros);
        i64::try_from(cost / 1_000_000).unwrap_or(i64::MAX)
    }
}

/// What the write step stores as its output.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WriteOutput {
    pub tool_calls: Vec<ToolCall>,
}

impl StepStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

impl TryFrom<&str> for StepStatus {
    type Error = capitainerie_common::CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            other => Err(capitainerie_common::CoreError::Internal(format!(
                "unknown step status {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_is_tokens_times_price_per_million() {
        let pricing = Pricing {
            input_per_million_micros: 150_000,
            output_per_million_micros: 600_000,
        };
        let usage = Usage {
            model: "m".into(),
            input_tokens: 2_000,
            output_tokens: 500,
        };
        assert_eq!(pricing.cost_micros(&usage), 300 + 300);
        assert_eq!(Pricing::default().cost_micros(&usage), 0);
    }
}
