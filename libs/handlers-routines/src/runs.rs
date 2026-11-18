use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use axum_extra::routing::TypedPath;
use capitainerie_common::RoutineId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::{RoutineRunListResponse, RoutineRunResponse};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/routines/{routine_id}/launch")]
pub struct LaunchRoutinePath {
    pub routine_id: RoutineId,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/routines/{routine_id}/runs")]
pub struct RoutineRunsPath {
    pub routine_id: RoutineId,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListRoutineRunsQuery {
    /// 1 to 100, 50 by default.
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[utoipa::path(
    post,
    path = "/api/v1/routines/{routine_id}/launch",
    operation_id = "launchRoutine",
    tag = super::TAG,
    params(("routine_id" = RoutineId, Path, description = "Routine identifier")),
    responses(
        (status = 202, description = "The passage: its card is created and the agent run started on behalf of the routine's creator, whatever the policies and triggers say. A run that cannot start leaves the card and gives the reason", body = RoutineRunResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the launch; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The routine is archived", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn launch_routine(
    LaunchRoutinePath { routine_id }: LaunchRoutinePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<(StatusCode, Json<RoutineRunResponse>), ApiError> {
    let run = state
        .usecase
        .launch_routine(caller.subject(), routine_id)
        .await?;
    Ok((StatusCode::ACCEPTED, Json(run.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/routines/{routine_id}/runs",
    operation_id = "listRoutineRuns",
    tag = super::TAG,
    params(
        ("routine_id" = RoutineId, Path, description = "Routine identifier"),
        ListRoutineRunsQuery,
    ),
    responses(
        (status = 200, description = "The routine's passages, newest first, each with where its card and agent run stand, with the total count", body = RoutineRunListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_routine_runs(
    RoutineRunsPath { routine_id }: RoutineRunsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListRoutineRunsQuery>,
) -> Result<Json<RoutineRunListResponse>, ApiError> {
    let (runs, total) = state
        .usecase
        .list_routine_runs(
            caller.subject(),
            routine_id,
            i64::from(query.limit.unwrap_or(50)),
            i64::from(query.offset.unwrap_or(0)),
        )
        .await?;
    Ok(Json(RoutineRunListResponse {
        items: runs.into_iter().map(Into::into).collect(),
        total,
    }))
}
