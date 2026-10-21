use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::IntegrationActionId;
use capitainerie_core::domain::integration::revert::RevertError;
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use super::{describe, dto::IntegrationActionListResponse};
use crate::{IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integration-actions/{id}/revert")]
pub struct RevertIntegrationActionPath {
    pub id: IntegrationActionId,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct RevertIntegrationActionRequest {
    #[serde(default)]
    pub confirm_production: bool,
}

fn failure(error: RevertError) -> IntegrationError {
    match error {
        RevertError::Core(error) => error.into(),
        RevertError::Refused(refusal) if refusal.is_conflict() => {
            IntegrationError::Api(ApiError::ConflictCode {
                code: refusal.code(),
                message: refusal.to_string(),
            })
        }
        RevertError::Refused(refusal) => IntegrationError::Unprocessable {
            code: refusal.code(),
            message: refusal.to_string(),
        },
        RevertError::Connector(failure) => IntegrationError::Unprocessable {
            code: failure.code(),
            message: failure.to_string(),
        },
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/integration-actions/{id}/revert",
    operation_id = "revertIntegrationAction",
    tag = crate::TAG,
    params(("id" = IntegrationActionId, Path, description = "Integration action identifier")),
    request_body = RevertIntegrationActionRequest,
    responses(
        (status = 200, description = "The restoration rows the journal gained, in the order they were written, each linked to the original through reverts_action_id; the original now carries reverted_by_action_id. When a write of the restoration fails, its row says so, the restoration stops there and the original stays unrestored", body = IntegrationActionListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the restoration; agents cannot restore", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such action in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "already_reverted, intervention_running, or changed_since: the state is no longer the one the action left, nothing was written", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "not_revertible, instance_archived, production_write_not_confirmed, or the instance could not be read: connection_failed, network_blocked, version_mismatch or not_supported", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    RevertIntegrationActionPath { id }: RevertIntegrationActionPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<RevertIntegrationActionRequest>,
) -> Result<Json<IntegrationActionListResponse>, IntegrationError> {
    let subject = caller.subject();
    let actions = state
        .usecase
        .revert_integration_action(
            subject.clone(),
            id,
            request.confirm_production,
            &state.connector,
        )
        .await
        .map_err(failure)?;
    Ok(Json(describe(&state.usecase, subject, actions).await?))
}
