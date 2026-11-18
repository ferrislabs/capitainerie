use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::McpConnectorId;
use capitainerie_core::domain::mcp::ports::{McpGateway, McpOAuth};
use capitainerie_handlers::{ApiError, Caller};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{response::McpToolResponse, state::McpState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/mcp-connectors/{connector_id}/test")]
pub struct TestMcpConnectorPath {
    pub connector_id: McpConnectorId,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestMcpConnectorResponse {
    pub tools: Vec<McpToolResponse>,
}

#[utoipa::path(
    post,
    path = "/api/v1/mcp-connectors/{connector_id}/test",
    operation_id = "testMcpConnector",
    tag = super::TAG,
    params(("connector_id" = McpConnectorId, Path, description = "Connector identifier")),
    responses(
        (status = 200, description = "The tools the server lists with the caller's own token", body = TestMcpConnectorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused it; agents may not connect", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such connector in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The caller is not connected, or the connection expired and must be redone", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The server could not be reached or refused the request", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "SECRETS_KEY is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    TestMcpConnectorPath { connector_id }: TestMcpConnectorPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<TestMcpConnectorResponse>, ApiError> {
    state.require_secrets()?;
    let tools = state
        .usecase
        .test_mcp_connector(
            caller.subject(),
            connector_id,
            &*state.adapter,
            &*state.adapter,
        )
        .await?;
    Ok(Json(TestMcpConnectorResponse {
        tools: tools.into_iter().map(Into::into).collect(),
    }))
}
