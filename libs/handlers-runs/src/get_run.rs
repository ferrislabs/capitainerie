use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::RunId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::RunResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/runs/{run_id}")]
pub struct RunPath {
    pub run_id: RunId,
}

#[utoipa::path(
    get,
    path = "/api/v1/runs/{run_id}",
    operation_id = "getRun",
    tag = super::TAG,
    params(("run_id" = RunId, Path, description = "Run identifier")),
    responses(
        (status = 200, description = "The run", body = RunResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such run in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    RunPath { run_id }: RunPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<RunResponse>, ApiError> {
    let run = state.usecase.get_run(caller.subject(), run_id).await?;
    Ok(Json(run.into()))
}
