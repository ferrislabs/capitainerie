use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::McpConnectorId;
use capitainerie_core::domain::mcp::ports::{McpGateway, McpOAuth};
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;

use crate::{response::McpConnectorResponse, state::McpState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/mcp-connectors/{connector_id}")]
pub struct McpConnectorPath {
    pub connector_id: McpConnectorId,
}

#[utoipa::path(
    get,
    path = "/api/v1/mcp-connectors/{connector_id}",
    operation_id = "getMcpConnector",
    tag = super::TAG,
    params(("connector_id" = McpConnectorId, Path, description = "Connector identifier")),
    responses(
        (status = 200, description = "The connector with the caller's connection state", body = McpConnectorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to connectors", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such connector in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    McpConnectorPath { connector_id }: McpConnectorPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<McpConnectorResponse>, ApiError> {
    let view = state
        .usecase
        .get_mcp_connector(caller.subject(), connector_id)
        .await?;
    Ok(Json(view.into()))
}
