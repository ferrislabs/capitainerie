use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::SourceId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::SourceResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/sources/{source_id}/archive")]
pub struct ArchiveSourcePath {
    pub source_id: SourceId,
}

#[utoipa::path(
    post,
    path = "/api/v1/sources/{source_id}/archive",
    operation_id = "archiveSource",
    tag = super::TAG,
    params(("source_id" = SourceId, Path, description = "Source identifier")),
    responses(
        (status = 200, description = "The archived source; its hook answers 404 from now on. Archiving twice changes nothing", body = SourceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage sources", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such source in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ArchiveSourcePath { source_id }: ArchiveSourcePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<SourceResponse>, ApiError> {
    let source = state
        .usecase
        .archive_source(caller.subject(), source_id)
        .await?;
    Ok(Json(source.into()))
}
