use axum::{Router, extract::State, http::StatusCode, routing::get};

use crate::{ApiError, AppState};

pub const TAG: &str = "health";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

#[utoipa::path(
    get,
    path = "/health/live",
    operation_id = "getLiveness",
    tag = TAG,
    responses((status = 204, description = "The process is up"))
)]
pub async fn live() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[utoipa::path(
    get,
    path = "/health/ready",
    operation_id = "getReadiness",
    tag = TAG,
    responses(
        (status = 204, description = "Dependencies are reachable"),
        (status = 503, description = "A dependency is down", body = crate::errors::ApiErrorResponse),
    )
)]
pub async fn ready(State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    state.usecase.readiness().await?;
    Ok(StatusCode::NO_CONTENT)
}
