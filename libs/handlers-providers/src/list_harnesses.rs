use axum::Json;
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::execution::harness::Harness;
use serde::Deserialize;

use crate::dto::{HarnessListResponse, HarnessResponse};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/harnesses")]
pub struct HarnessesPath;

#[utoipa::path(
    get,
    path = "/api/v1/harnesses",
    operation_id = "listHarnesses",
    tag = super::TAG,
    responses(
        (status = 200, description = "The coding agents an execution can run on, with the provider whose key each one uses and the models it accepts", body = HarnessListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(_: HarnessesPath) -> Json<HarnessListResponse> {
    Json(HarnessListResponse {
        harnesses: Harness::ALL
            .into_iter()
            .map(HarnessResponse::from)
            .collect(),
    })
}
