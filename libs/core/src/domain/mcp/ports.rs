use capitainerie_common::{CoreError, McpConnectorId, OrganizationId};
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{
    CodeExchange, Discovery, McpConnection, McpConnector, PendingAuthorization, RefreshExchange,
    TokenSet, ToolInfo, ToolOutput,
};
use crate::domain::agent::SecretToken;

pub type ConnectionExpiry = (McpConnectorId, Option<DateTime<Utc>>);

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait McpConnectorRepository: Send {
    fn insert(
        &mut self,
        connector: &McpConnector,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: McpConnectorId,
    ) -> impl Future<Output = Result<Option<McpConnector>, CoreError>> + Send;

    fn update(
        &mut self,
        connector: &McpConnector,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> impl Future<Output = Result<Vec<McpConnector>, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait McpConnectionRepository: Send {
    fn find(
        &mut self,
        organization_id: OrganizationId,
        connector_id: McpConnectorId,
        subject: &str,
    ) -> impl Future<Output = Result<Option<McpConnection>, CoreError>> + Send;

    fn upsert(
        &mut self,
        organization_id: OrganizationId,
        connection: &McpConnection,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn delete(
        &mut self,
        organization_id: OrganizationId,
        connector_id: McpConnectorId,
        subject: &str,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn expiries(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> impl Future<Output = Result<Vec<ConnectionExpiry>, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait McpAuthorizationRepository: Send {
    fn insert(
        &mut self,
        organization_id: OrganizationId,
        pending: &PendingAuthorization,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn take(
        &mut self,
        organization_id: OrganizationId,
        state: &str,
    ) -> impl Future<Output = Result<Option<PendingAuthorization>, CoreError>> + Send;

    fn purge_expired(
        &mut self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

pub trait McpOAuth: Send + Sync {
    fn discover(&self, url: &str) -> impl Future<Output = Result<Discovery, CoreError>> + Send;

    fn register_client(
        &self,
        registration_endpoint: &str,
        redirect_uri: &str,
        scopes: &[String],
    ) -> impl Future<Output = Result<String, CoreError>> + Send;

    fn exchange_code(
        &self,
        request: &CodeExchange<'_>,
    ) -> impl Future<Output = Result<TokenSet, CoreError>> + Send;

    fn refresh(
        &self,
        request: &RefreshExchange<'_>,
    ) -> impl Future<Output = Result<TokenSet, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait McpGateway: Send + Sync {
    fn list_tools(
        &self,
        url: &str,
        token: &SecretToken,
    ) -> impl Future<Output = Result<Vec<ToolInfo>, CoreError>> + Send;

    fn call_tool(
        &self,
        url: &str,
        token: &SecretToken,
        name: &str,
        arguments: &Value,
    ) -> impl Future<Output = Result<ToolOutput, CoreError>> + Send;
}
