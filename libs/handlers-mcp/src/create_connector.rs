use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_core::domain::mcp::{
    ConnectorView,
    ports::{McpGateway, McpOAuth},
    service::CreateConnector,
};
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{list_connectors::McpConnectorsPath, response::McpConnectorResponse, state::McpState};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMcpConnectorRequest {
    /// The MCP server endpoint, e.g. https://demo.mestier.fr/mcp.
    pub url: String,
    /// Defaults to the server's host name.
    pub name: Option<String>,
    /// Kebab-case, unique in the organization; defaults to the name's slug.
    pub slug: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/mcp-connectors",
    operation_id = "createMcpConnector",
    tag = super::TAG,
    request_body = CreateMcpConnectorRequest,
    responses(
        (status = 201, description = "The connector: the server's OAuth metadata was discovered and a public client registered", body = McpConnectorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage connectors", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A connector of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An invalid field, or a server that cannot be discovered or registered with", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "CONSOLE_URL is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: McpOAuth + McpGateway + 'static>(
    _: McpConnectorsPath,
    State(state): State<McpState<A>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateMcpConnectorRequest>,
) -> Result<(StatusCode, Json<McpConnectorResponse>), ApiError> {
    let redirect_uri = state.redirect_uri()?;
    let connector = state
        .usecase
        .create_mcp_connector(
            caller.subject(),
            CreateConnector {
                url: request.url,
                name: request.name,
                slug: request.slug,
            },
            &redirect_uri,
            &*state.adapter,
        )
        .await?;
    let view = ConnectorView {
        connector,
        connection: None,
    };
    Ok((StatusCode::CREATED, Json(view.into())))
}
