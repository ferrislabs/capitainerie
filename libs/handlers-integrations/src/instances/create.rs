use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_core::domain::{
    agent::SecretToken,
    integration::{Environment, IntegrationKind, service::CreateInstance},
};
use capitainerie_handlers::Caller;

use super::{
    dto::{CreateIntegrationInstanceRequest, IntegrationInstanceResponse},
    list::ClientIntegrationsPath,
};
use crate::{IntegrationError, state::IntegrationsState};

#[utoipa::path(
    post,
    path = "/api/v1/clients/{client_id}/integrations",
    operation_id = "createIntegrationInstance",
    tag = crate::TAG,
    params(("client_id" = capitainerie_common::ClientId, Path, description = "Client identifier")),
    request_body = CreateIntegrationInstanceRequest,
    responses(
        (status = 201, description = "The instance, stored once the connection test passed; the secret is sealed and never returned", body = IntegrationInstanceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such client in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The client already has an instance of that name (instance_name_taken)", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Invalid fields, or the connection test failed: connection_failed, version_mismatch, network_blocked or not_supported", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ClientIntegrationsPath { client_id }: ClientIntegrationsPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateIntegrationInstanceRequest>,
) -> Result<(StatusCode, Json<IntegrationInstanceResponse>), IntegrationError> {
    let instance = state
        .usecase
        .create_integration_instance(
            caller.subject(),
            CreateInstance {
                client_id,
                kind: IntegrationKind::from(request.kind).as_str().to_owned(),
                name: request.name,
                environment: Environment::from(request.environment).as_str().to_owned(),
                base_url: request.base_url,
                database: request.database,
                login: request.login,
                secret: SecretToken::new(request.secret),
            },
            &state.connector,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(instance.into())))
}
