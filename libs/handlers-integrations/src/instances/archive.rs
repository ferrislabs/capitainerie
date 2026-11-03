use axum::{Extension, Json, extract::State};
use capitainerie_common::{CoreError, IntegrationInstanceId};
use capitainerie_core::domain::integration::service::INSTANCE_IN_USE;
use capitainerie_handlers::{ApiError, Caller};

use super::{dto::IntegrationInstanceResponse, update::IntegrationInstancePath};
use crate::{IntegrationError, state::IntegrationsState};

fn failure(error: CoreError) -> IntegrationError {
    match error {
        CoreError::Conflict(code) if code == INSTANCE_IN_USE => {
            IntegrationError::Api(ApiError::ConflictCode {
                code: INSTANCE_IN_USE,
                message: "a queued or running intervention holds a grant on this instance".into(),
            })
        }
        other => other.into(),
    }
}

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
        (status = 409, description = "instance_in_use: a queued or running intervention holds a grant on this instance", body = capitainerie_handlers::errors::ApiErrorResponse),
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
        .await
        .map_err(failure)?;
    Ok(Json(instance.into()))
}
