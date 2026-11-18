use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::mcp::{
    ports::{McpGateway, McpOAuth},
    service::UpdateConnector,
};
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{get_connector::McpConnectorPath, response::McpConnectorResponse, state::McpState};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMcpConnectorRequest {
    pub name: Option<String>,
    pub archived: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/v1/mcp-connectors/{connector_id}",
    operation_id = "updateMcpConnector",
    tag = super::TAG,
    params(("connector_id" = capitainerie_common::McpConnectorId, Path, description = "Connector identifier")),
    request_body = UpdateMcpConnectorRequest,
    responses(
        (status = 200, description = "The updated connector; absent fields are unchanged", body = McpConnectorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage connectors", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such connector in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, or an invalid field", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    McpConnectorPath { connector_id }: McpConnectorPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateMcpConnectorRequest>,
) -> Result<Json<McpConnectorResponse>, ApiError> {
    state
        .usecase
        .update_mcp_connector(
            caller.subject(),
            connector_id,
            UpdateConnector {
                name: request.name,
                archived: request.archived,
            },
        )
        .await?;
    let view = state
        .usecase
        .get_mcp_connector(caller.subject(), connector_id)
        .await?;
    Ok(Json(view.into()))
}
