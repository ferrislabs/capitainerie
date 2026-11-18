use capitainerie_common::McpConnectorId;
use capitainerie_core::domain::mcp::{ConnectorView, ToolInfo};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct McpConnectorResponse {
    pub id: McpConnectorId,
    pub slug: String,
    pub name: String,
    /// The MCP server endpoint.
    pub url: String,
    /// The RFC 8707 resource the tokens are requested for.
    pub resource: String,
    pub issuer: String,
    pub scopes: Vec<String>,
    pub archived: bool,
    /// Whether the caller holds a token for this connector.
    pub connected: bool,
    /// When the caller's access token expires, if it does.
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpConnectorListResponse {
    pub items: Vec<McpConnectorResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpToolResponse {
    pub name: String,
    pub read_only: bool,
}

impl From<ConnectorView> for McpConnectorResponse {
    fn from(view: ConnectorView) -> Self {
        let connector = view.connector;
        Self {
            id: connector.id,
            slug: connector.slug.as_str().to_owned(),
            name: connector.name,
            url: connector.url,
            resource: connector.resource,
            issuer: connector.issuer,
            scopes: connector.scopes,
            archived: connector.archived,
            connected: view.connection.is_some(),
            expires_at: view.connection.and_then(|connection| connection.expires_at),
            created_by: connector.created_by,
            created_at: connector.created_at,
            updated_at: connector.updated_at,
        }
    }
}

impl From<ToolInfo> for McpToolResponse {
    fn from(tool: ToolInfo) -> Self {
        Self {
            read_only: tool.read_only_hint.unwrap_or(false),
            name: tool.name,
        }
    }
}
