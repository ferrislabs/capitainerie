use axum::{Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, AppState};
use serde::Deserialize;

use crate::response::RoutineRunResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/hooks/routines/{token}")]
pub struct RoutineHookPath {
    pub token: String,
}

#[utoipa::path(
    post,
    path = "/hooks/routines/{token}",
    operation_id = "triggerRoutine",
    tag = super::TAG,
    params(("token" = String, Path, description = "The webhook trigger's token")),
    responses(
        (status = 202, description = "The passage: card created and agent run started, or queued or skipped when the previous card is still open, as the routine's if_open says. The body of the request is ignored", body = RoutineRunResponse),
        (status = 404, description = "No enabled webhook trigger of an active routine has this token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
)]
pub async fn trigger_routine(
    RoutineHookPath { token }: RoutineHookPath,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<RoutineRunResponse>), ApiError> {
    let run = state.usecase.receive_routine_hook(&token).await?;
    Ok((StatusCode::ACCEPTED, Json(run.into())))
}
