use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::run::active::ActiveRun;
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::response::RunResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/runs/active")]
pub struct ActiveRunsPath;

#[derive(Debug, Serialize, ToSchema)]
pub struct ActiveRunListResponse {
    /// Queued and running runs, oldest first, 100 at most.
    pub items: Vec<ActiveRunResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActiveRunResponse {
    pub run: RunResponse,
    /// The latest running step; absent while the run waits in the queue.
    pub current_step: Option<ActiveStepResponse>,
    pub steps_done: u32,
    /// Length of the run's pipeline.
    pub steps_total: u32,
    /// Sum over the steps, finished or not.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActiveStepResponse {
    /// extract, deduplicate, write or execute.
    pub kind: String,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub started_at: DateTime<Utc>,
}

impl From<ActiveRun> for ActiveRunResponse {
    fn from(active: ActiveRun) -> Self {
        Self {
            run: active.overview.into(),
            current_step: active.current_step.map(|step| ActiveStepResponse {
                kind: step.kind.as_str().to_owned(),
                agent: step.agent,
                model: (!step.usage.model.is_empty()).then_some(step.usage.model),
                started_at: step.started_at,
            }),
            steps_done: active.steps_done,
            steps_total: active.steps_total,
            input_tokens: active.input_tokens,
            output_tokens: active.output_tokens,
            cost_micros: active.cost_micros,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/runs/active",
    operation_id = "listActiveRuns",
    tag = super::TAG,
    responses(
        (status = 200, description = "The organization's queued and running runs, oldest first", body = ActiveRunListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: ActiveRunsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ActiveRunListResponse>, ApiError> {
    let runs = state.usecase.list_active_runs(caller.subject()).await?;
    Ok(Json(ActiveRunListResponse {
        items: runs.into_iter().map(ActiveRunResponse::from).collect(),
    }))
}
