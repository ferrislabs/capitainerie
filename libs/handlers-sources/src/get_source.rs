use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::SourceId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::SourceResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/sources/{source_id}")]
pub struct SourcePath {
    pub source_id: SourceId,
}

#[utoipa::path(
    get,
    path = "/api/v1/sources/{source_id}",
    operation_id = "getSource",
    tag = super::TAG,
    params(("source_id" = SourceId, Path, description = "Source identifier")),
    responses(
        (status = 200, description = "The source; its token is never shown again", body = SourceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to sources", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such source in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    SourcePath { source_id }: SourcePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<SourceResponse>, ApiError> {
    let source = state
        .usecase
        .get_source(caller.subject(), source_id)
        .await?;
    Ok(Json(source.into()))
}
