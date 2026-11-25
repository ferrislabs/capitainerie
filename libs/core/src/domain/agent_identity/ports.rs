use capitainerie_common::{AgentId, CoreError, OrganizationId};

use super::{AgentClientId, AgentIdentity, ProvisionedClient};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait AgentIdentityRepository: Send {
    fn find(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
    ) -> impl Future<Output = Result<Option<AgentIdentity>, CoreError>> + Send;

    fn upsert(
        &mut self,
        identity: &AgentIdentity,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Pending identities of every organization, least recently tried first.
    fn pending(
        &mut self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<AgentIdentity>, CoreError>> + Send;
}

/// Manages agents' clients in the organization's identity provider.
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait AgentIdentityProvisioner: Send + Sync {
    /// Creates the client, or completes the one already there: confidential,
    /// service account, token exchange on, the API allowed to address it and
    /// the agent allowed to delegate to the MCP server. Calling it again
    /// changes nothing.
    fn provision(
        &self,
        issuer: &str,
        client_id: &AgentClientId,
        name: &str,
    ) -> impl Future<Output = Result<ProvisionedClient, CoreError>> + Send;

    fn set_enabled(
        &self,
        issuer: &str,
        client_uuid: &str,
        enabled: bool,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
