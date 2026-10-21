use axum::{Extension, Json, extract::State};
use capitainerie_common::IntegrationInstanceId;
use capitainerie_handlers::Caller;

use super::{dto::IntegrationInstanceResponse, update::IntegrationInstancePath};
use crate::{IntegrationError, state::IntegrationsState};

#[utoipa::path(
    delete,
    path = "/api/v1/integration-instances/{id}",
    operation_id = "archiveIntegrationInstance",
    tag = crate::TAG,
    params(("id" = IntegrationInstanceId, Path, description = "Integration instance identifier")),
    responses(
        (status = 200, description = "The archived instance; archiving an archived instance changes nothing", body = IntegrationInstanceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such instance in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    IntegrationInstancePath { id }: IntegrationInstancePath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<IntegrationInstanceResponse>, IntegrationError> {
    let instance = state
        .usecase
        .archive_integration_instance(caller.subject(), id)
        .await?;
    Ok(Json(instance.into()))
}
