use capitainerie_common::{AgentId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use super::{AgentClientId, AgentIdentity, IdentitySummary, ProvisionedClient, UnsyncedClient};

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

    /// Writes the identity only if its stored `updated_at` is still
    /// `expected`. False when someone changed it in between, an archive
    /// during a sweep for instance: that change wins.
    fn upsert_unchanged(
        &mut self,
        identity: &AgentIdentity,
        expected: DateTime<Utc>,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// Reads the identity without opening its secret.
    fn summary(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
    ) -> impl Future<Output = Result<Option<IdentitySummary>, CoreError>> + Send;

    /// Pending identities of every organization, least recently changed
    /// first: the ones never tried, and the ones last tried before
    /// `retry_before`.
    fn pending(
        &mut self,
        retry_before: DateTime<Utc>,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<AgentIdentity>, CoreError>> + Send;

    /// Clients of every organization FerrisKey has yet to enable or disable,
    /// least recently changed first.
    fn unsynced(
        &mut self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<UnsyncedClient>, CoreError>> + Send;

    /// Records that FerrisKey follows the identity, unless it changed since
    /// `expected`. False in that case: the change waits for the next sweep.
    fn mark_synced(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
        expected: DateTime<Utc>,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// Gives an identity to every agent that has none: pending, or disabled
    /// when the agent is archived. Returns how many were added.
    fn adopt(&mut self, now: DateTime<Utc>) -> impl Future<Output = Result<u64, CoreError>> + Send;
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
