use axum::{Extension, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::McpConnectorId;
use capitainerie_core::domain::mcp::ports::{McpGateway, McpOAuth};
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;

use crate::state::McpState;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/mcp-connectors/{connector_id}/connection")]
pub struct McpConnectionPath {
    pub connector_id: McpConnectorId,
}

#[utoipa::path(
    delete,
    path = "/api/v1/mcp-connectors/{connector_id}/connection",
    operation_id = "disconnectMcpConnector",
    tag = super::TAG,
    params(("connector_id" = McpConnectorId, Path, description = "Connector identifier")),
    responses(
        (status = 204, description = "The caller's tokens were deleted; already disconnected is fine"),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused it; agents may not connect", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such connector in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    McpConnectionPath { connector_id }: McpConnectionPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
) -> Result<StatusCode, ApiError> {
    state
        .usecase
        .disconnect_mcp_connector(caller.subject(), connector_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
