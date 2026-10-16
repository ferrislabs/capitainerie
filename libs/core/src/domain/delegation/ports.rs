use capitainerie_common::{CoreError, RunId};

use super::DelegatedToken;
use crate::domain::agent::SecretToken;

/// Token exchange (RFC 8693) against the organization's realm.
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait TokenExchanger: Send + Sync {
    /// API side: turns the user's token into one addressed to `agent_client_id`.
    fn for_agent(
        &self,
        issuer: &str,
        user_token: &SecretToken,
        agent_client_id: &str,
    ) -> impl Future<Output = Result<DelegatedToken, CoreError>> + Send;

    /// Agent side: the agent presents itself (client credentials) and gets a
    /// delegated token (`act` = agent) addressed to the Kanban MCP.
    fn for_mcp(
        &self,
        issuer: &str,
        agent_token: &SecretToken,
        agent_client_id: &str,
    ) -> impl Future<Output = Result<DelegatedToken, CoreError>> + Send;
}

/// Where the API leaves the agent-addressed token until the worker picks the
/// run up. Interim, until FerrisKey offers a durable delegation grant.
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait DelegationVault: Send {
    fn store(
        &mut self,
        run_id: RunId,
        agent_client_id: &str,
        token: &DelegatedToken,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn load(
        &mut self,
        run_id: RunId,
        agent_client_id: &str,
    ) -> impl Future<Output = Result<Option<DelegatedToken>, CoreError>> + Send;

    fn purge(&mut self, run_id: RunId) -> impl Future<Output = Result<(), CoreError>> + Send;
}
