use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::AgentId;
use capitainerie_core::domain::run::RunFilter;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::response::{RunResponse, RunStatusResponse};

#[derive(TypedPath)]
#[typed_path("/api/v1/runs")]
pub struct RunsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListRunsQuery {
    /// 1 to 100, 20 by default.
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<RunStatusResponse>,
    /// Only the agent task runs of this agent.
    pub agent_id: Option<AgentId>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunListResponse {
    pub items: Vec<RunResponse>,
    pub total: u64,
}

#[utoipa::path(
    get,
    path = "/api/v1/runs",
    operation_id = "listRuns",
    tag = super::TAG,
    params(ListRunsQuery),
    responses(
        (status = 200, description = "The organization's runs, newest first, narrowed by status and agent; `total` counts the matching runs", body = RunListResponse),
        (status = 400, description = "An unknown status or a malformed agent id", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: RunsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListRunsQuery>,
) -> Result<Json<RunListResponse>, ApiError> {
    let (runs, total) = state
        .usecase
        .list_runs(
            caller.subject(),
            RunFilter {
                status: query.status.map(Into::into),
                agent_id: query.agent_id,
            },
            query.limit.unwrap_or(20),
            query.offset.unwrap_or(0),
        )
        .await?;
    Ok(Json(RunListResponse {
        items: runs.into_iter().map(RunResponse::from).collect(),
        total,
    }))
}
