use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::McpConnectorId;
use capitainerie_core::domain::mcp::ports::{McpGateway, McpOAuth};
use capitainerie_handlers::{ApiError, Caller};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::McpState;

#[derive(TypedPath)]
#[typed_path("/api/v1/mcp-connectors/oauth/callback")]
pub struct McpOAuthCallbackPath;

#[derive(Debug, Deserialize, ToSchema)]
pub struct McpOAuthCallbackRequest {
    pub state: String,
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpOAuthCallbackResponse {
    pub connector_id: McpConnectorId,
}

#[utoipa::path(
    post,
    path = "/api/v1/mcp-connectors/oauth/callback",
    operation_id = "completeMcpOAuth",
    tag = super::TAG,
    request_body = McpOAuthCallbackRequest,
    responses(
        (status = 200, description = "The code was exchanged and the caller's tokens stored, sealed", body = McpOAuthCallbackResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The state was issued to another user, or the policy refused it", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Unknown, used or expired state, or a code the server refused", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "SECRETS_KEY is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    _: McpOAuthCallbackPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<McpOAuthCallbackRequest>,
) -> Result<Json<McpOAuthCallbackResponse>, ApiError> {
    state.require_secrets()?;
    let connector_id = state
        .usecase
        .complete_mcp_authorization(
            caller.subject(),
            &request.state,
            &request.code,
            &*state.adapter,
        )
        .await?;
    Ok(Json(McpOAuthCallbackResponse { connector_id }))
}
