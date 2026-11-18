use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::McpConnectorId;
use capitainerie_core::domain::mcp::ports::{McpGateway, McpOAuth};
use capitainerie_handlers::{ApiError, Caller};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::McpState;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/mcp-connectors/{connector_id}/authorize")]
pub struct AuthorizeMcpConnectorPath {
    pub connector_id: McpConnectorId,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthorizeMcpConnectorResponse {
    /// Where to send the user's browser: authorization code with PKCE S256.
    pub authorization_url: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/mcp-connectors/{connector_id}/authorize",
    operation_id = "authorizeMcpConnector",
    tag = super::TAG,
    params(("connector_id" = McpConnectorId, Path, description = "Connector identifier")),
    responses(
        (status = 200, description = "The authorization URL; its state is good for ten minutes and one callback", body = AuthorizeMcpConnectorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused it; agents may not connect", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such connector in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The connector is archived", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "SECRETS_KEY or CONSOLE_URL is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    AuthorizeMcpConnectorPath { connector_id }: AuthorizeMcpConnectorPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<AuthorizeMcpConnectorResponse>, ApiError> {
    state.require_secrets()?;
    let redirect_uri = state.redirect_uri()?;
    let start = state
        .usecase
        .start_mcp_authorization(caller.subject(), connector_id, &redirect_uri)
        .await?;
    Ok(Json(AuthorizeMcpConnectorResponse {
        authorization_url: start.authorization_url,
    }))
}
