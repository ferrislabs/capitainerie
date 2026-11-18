use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::SourceId;
use capitainerie_handlers::{AppState, Caller};
use serde::Deserialize;

use crate::{hook::HookError, response::SourceEventResponse};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/sources/{source_id}/test")]
pub struct TestSourcePath {
    pub source_id: SourceId,
}

#[utoipa::path(
    post,
    path = "/api/v1/sources/{source_id}/test",
    operation_id = "testSource",
    tag = super::TAG,
    params(("source_id" = SourceId, Path, description = "Source identifier")),
    responses(
        (status = 202, description = "A sample event went through the same path as the hook, even on a disabled source: its card is created and, when the source asks for it, its run queued", body = SourceEventResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage sources", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such source in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The source is archived", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 429, description = "The source received too many events in the last minute", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    TestSourcePath { source_id }: TestSourcePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<(StatusCode, Json<SourceEventResponse>), HookError> {
    let event = state
        .usecase
        .test_source(caller.subject(), source_id)
        .await?
        .map_err(HookError::Refused)?;
    Ok((StatusCode::ACCEPTED, Json(event.into())))
}
