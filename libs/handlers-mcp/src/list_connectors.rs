use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::mcp::ports::{McpGateway, McpOAuth};
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{response::McpConnectorListResponse, state::McpState};

#[derive(TypedPath)]
#[typed_path("/api/v1/mcp-connectors")]
pub struct McpConnectorsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListMcpConnectorsQuery {
    pub archived: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/v1/mcp-connectors",
    operation_id = "listMcpConnectors",
    tag = super::TAG,
    params(ListMcpConnectorsQuery),
    responses(
        (status = 200, description = "The organization's MCP connectors by name, each with the caller's connection state; without archived, the active ones only", body = McpConnectorListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to connectors", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    _: McpConnectorsPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListMcpConnectorsQuery>,
) -> Result<Json<McpConnectorListResponse>, ApiError> {
    let views = state
        .usecase
        .list_mcp_connectors(caller.subject(), query.archived)
        .await?;
    Ok(Json(McpConnectorListResponse {
        items: views.into_iter().map(Into::into).collect(),
    }))
}
