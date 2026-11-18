use axum::{Extension, Json, extract::State, http::HeaderMap};
use axum_extra::routing::TypedPath;
use capitainerie_common::SourceId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::{PublicBaseUrl, response::IssuedSourceResponse};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/sources/{source_id}/rotate-token")]
pub struct RotateTokenPath {
    pub source_id: SourceId,
}

#[utoipa::path(
    post,
    path = "/api/v1/sources/{source_id}/rotate-token",
    operation_id = "rotateSourceToken",
    tag = super::TAG,
    params(("source_id" = SourceId, Path, description = "Source identifier")),
    responses(
        (status = 200, description = "The new token and webhook URL, shown this once; the previous URL stops working at once", body = IssuedSourceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage sources", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such source in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    RotateTokenPath { source_id }: RotateTokenPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(base): Extension<PublicBaseUrl>,
    headers: HeaderMap,
) -> Result<Json<IssuedSourceResponse>, ApiError> {
    let issued = state
        .usecase
        .rotate_source_token(caller.subject(), source_id)
        .await?;
    Ok(Json(IssuedSourceResponse::new(issued, &base, &headers)))
}
