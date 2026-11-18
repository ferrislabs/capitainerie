use capitainerie_common::{AgentId, CoreError, OrganizationId};

use super::CrewAgent;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait CrewRepository: Send {
    fn insert(&mut self, agent: &CrewAgent) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: AgentId,
    ) -> impl Future<Output = Result<Option<CrewAgent>, CoreError>> + Send;

    fn update(&mut self, agent: &CrewAgent) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// `archived` of `None` lists every agent.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> impl Future<Output = Result<Vec<CrewAgent>, CoreError>> + Send;
}
