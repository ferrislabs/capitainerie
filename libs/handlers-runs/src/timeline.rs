use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::RunId;
use capitainerie_core::{
    RunTimeline,
    domain::{
        authorization::DecisionRecord,
        run::{
            StepKind,
            step::{Step, StepStatus, WriteOutput},
        },
    },
};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::response::RunResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/runs/{run_id}/timeline")]
pub struct RunTimelinePath {
    pub run_id: RunId,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunTimelineResponse {
    pub run: RunResponse,
    pub steps: Vec<StepResponse>,
    pub decisions: Vec<DecisionResponse>,
    pub totals: UsageTotals,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StepStatusResponse {
    Running,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StepResponse {
    pub seq: i32,
    pub kind: String,
    pub status: StepStatusResponse,
    /// The agent that ran the step: "extractor", or an agent client id.
    pub agent: Option<String>,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
    /// A one-line account of what the step did or why it was skipped.
    pub detail: Option<String>,
    pub tool_calls: Vec<ToolCallResponse>,
    /// What an `execute` step produced; absent for the other steps.
    pub execution: Option<ExecutionResponse>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct ExecutionResponse {
    pub summary: Option<String>,
    pub branch: Option<String>,
    pub sha: Option<String>,
    pub pull_request: Option<String>,
    #[serde(default)]
    pub no_changes: bool,
    #[serde(default)]
    pub dropped_tool_calls: u64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ToolCallResponse {
    pub tool: String,
    pub succeeded: bool,
    pub latency_ms: u64,
    pub summary: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DecisionResponse {
    pub action: String,
    pub actor: Option<String>,
    pub resource_kind: String,
    pub resource_id: Option<Uuid>,
    pub allowed: bool,
    pub reason: Option<String>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct UsageTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
}

impl From<RunTimeline> for RunTimelineResponse {
    fn from(timeline: RunTimeline) -> Self {
        let totals = timeline
            .steps
            .iter()
            .fold(UsageTotals::default(), |mut totals, step| {
                totals.input_tokens += step.usage.input_tokens;
                totals.output_tokens += step.usage.output_tokens;
                totals.cost_micros += step.cost_micros;
                totals
            });
        Self {
            run: timeline.run.into(),
            steps: timeline.steps.into_iter().map(StepResponse::from).collect(),
            decisions: timeline
                .decisions
                .into_iter()
                .map(DecisionResponse::from)
                .collect(),
            totals,
        }
    }
}

impl From<Step> for StepResponse {
    fn from(step: Step) -> Self {
        let tool_calls = step
            .output
            .as_ref()
            .and_then(|output| serde_json::from_value::<WriteOutput>(output.clone()).ok())
            .map(|output| {
                output
                    .tool_calls
                    .into_iter()
                    .map(|call| ToolCallResponse {
                        tool: call.tool,
                        succeeded: call.succeeded,
                        latency_ms: call.latency_ms,
                        summary: call.summary,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let execution = (step.kind == StepKind::Execute)
            .then(|| {
                step.output
                    .as_ref()
                    .and_then(|output| serde_json::from_value(output.clone()).ok())
            })
            .flatten();
        Self {
            seq: step.seq,
            kind: step.kind.as_str().to_owned(),
            status: match step.status {
                StepStatus::Running => StepStatusResponse::Running,
                StepStatus::Succeeded => StepStatusResponse::Succeeded,
                StepStatus::Failed => StepStatusResponse::Failed,
                StepStatus::Skipped => StepStatusResponse::Skipped,
            },
            detail: step.output.as_ref().and_then(detail),
            agent: step.agent,
            model: (!step.usage.model.is_empty()).then_some(step.usage.model),
            input_tokens: step.usage.input_tokens,
            output_tokens: step.usage.output_tokens,
            cost_micros: step.cost_micros,
            latency_ms: step.latency_ms,
            error: step.error,
            tool_calls,
            execution,
            started_at: step.started_at,
            finished_at: step.finished_at,
        }
    }
}

fn detail(output: &serde_json::Value) -> Option<String> {
    if let Some(reason) = output.get("reason").and_then(|reason| reason.as_str()) {
        return Some(reason.to_owned());
    }
    let actions = output.get("actions")?.as_array()?.len();
    let unlocated = output
        .get("unlocated")
        .and_then(|value| value.as_array())
        .map_or(0, Vec::len);
    Some(format!(
        "{actions} action(s) anchored in the transcript, {unlocated} dropped (quote not found)"
    ))
}

impl From<DecisionRecord> for DecisionResponse {
    fn from(record: DecisionRecord) -> Self {
        Self {
            action: record.action,
            actor: record.actor,
            resource_kind: record.resource_kind,
            resource_id: record.resource_id,
            allowed: record.allowed,
            reason: record.reason,
            decided_at: record.decided_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/runs/{run_id}/timeline",
    operation_id = "getRunTimeline",
    tag = super::TAG,
    params(("run_id" = RunId, Path, description = "Run identifier")),
    responses(
        (status = 200, description = "Steps, tool calls and policy decisions of the run", body = RunTimelineResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such run in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    RunTimelinePath { run_id }: RunTimelinePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<RunTimelineResponse>, ApiError> {
    let timeline = state.usecase.run_timeline(caller.subject(), run_id).await?;
    Ok(Json(timeline.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_execute_step_output_reads_as_an_execution() {
        let output = serde_json::json!({
            "tool_calls": [],
            "dropped_tool_calls": 3,
            "summary": "Ajout du CHANGELOG.",
            "branch": "capitainerie/0a1b2c3d4e5f",
            "sha": "abc123",
            "pull_request": "https://github.com/acme/web/pull/7",
            "no_changes": false,
            "job_started": true,
            "error": null
        });

        let execution: ExecutionResponse = serde_json::from_value(output).unwrap();

        assert_eq!(
            execution.branch.as_deref(),
            Some("capitainerie/0a1b2c3d4e5f")
        );
        assert_eq!(
            execution.pull_request.as_deref(),
            Some("https://github.com/acme/web/pull/7")
        );
        assert_eq!(execution.dropped_tool_calls, 3);
        assert!(!execution.no_changes);
    }
}
