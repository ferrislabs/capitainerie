use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::IntegrationInstanceId;
use capitainerie_handlers::Caller;
use serde::Deserialize;

use super::dto::CheckIntegrationInstanceResponse;
use crate::{IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integration-instances/{id}/check")]
pub struct CheckIntegrationInstancePath {
    pub id: IntegrationInstanceId,
}

#[utoipa::path(
    post,
    path = "/api/v1/integration-instances/{id}/check",
    operation_id = "checkIntegrationInstance",
    tag = crate::TAG,
    params(("id" = IntegrationInstanceId, Path, description = "Integration instance identifier")),
    responses(
        (status = 200, description = "The outcome of the connection test, failed or not; the date of the test is recorded, and the known server version kept when it failed", body = CheckIntegrationInstanceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such instance in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The instance is archived", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    CheckIntegrationInstancePath { id }: CheckIntegrationInstancePath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<CheckIntegrationInstanceResponse>, IntegrationError> {
    let (instance, outcome) = state
        .usecase
        .check_integration_instance(caller.subject(), id, &state.connector)
        .await?;
    Ok(Json(CheckIntegrationInstanceResponse::new(
        instance, outcome,
    )))
}
