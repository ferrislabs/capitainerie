use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::IntegrationInstanceId;
use capitainerie_core::domain::{
    agent::SecretToken,
    integration::{Environment, service::UpdateInstance},
};
use capitainerie_handlers::Caller;
use serde::Deserialize;

use super::dto::{IntegrationInstanceResponse, UpdateIntegrationInstanceRequest};
use crate::{IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integration-instances/{id}")]
pub struct IntegrationInstancePath {
    pub id: IntegrationInstanceId,
}

#[utoipa::path(
    patch,
    path = "/api/v1/integration-instances/{id}",
    operation_id = "updateIntegrationInstance",
    tag = crate::TAG,
    params(("id" = IntegrationInstanceId, Path, description = "Integration instance identifier")),
    request_body = UpdateIntegrationInstanceRequest,
    responses(
        (status = 200, description = "The updated instance; absent fields are unchanged, and a new URL, database, login or secret is tested first. A new origin (scheme, host or port), database or login needs a new secret", body = IntegrationInstanceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such instance in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The client already has an instance of that name (instance_name_taken)", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, invalid fields, a new origin, database or login without a new secret, an archived instance, or the connection test failed: connection_failed, version_mismatch, network_blocked or not_supported", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    IntegrationInstancePath { id }: IntegrationInstancePath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateIntegrationInstanceRequest>,
) -> Result<Json<IntegrationInstanceResponse>, IntegrationError> {
    let instance = state
        .usecase
        .update_integration_instance(
            caller.subject(),
            id,
            UpdateInstance {
                name: request.name,
                environment: request
                    .environment
                    .map(|environment| Environment::from(environment).as_str().to_owned()),
                base_url: request.base_url,
                database: request.database,
                login: request.login,
                secret: request.secret.map(SecretToken::new),
            },
            &state.connector,
        )
        .await?;
    Ok(Json(instance.into()))
}
