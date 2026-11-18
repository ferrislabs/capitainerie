use capitainerie_common::{AgentId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use super::{Budget, BudgetSettings, Spend};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait BudgetRepository: Send {
    /// With the agents' names.
    fn list(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Budget>, CoreError>> + Send;

    /// Replaces every budget of the organization.
    fn replace(
        &mut self,
        organization_id: OrganizationId,
        settings: &BudgetSettings,
        by: &str,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// How many of `agents` belong to the organization.
    fn count_agents(
        &mut self,
        organization_id: OrganizationId,
        agents: &[AgentId],
    ) -> impl Future<Output = Result<i64, CoreError>> + Send;

    /// The cost of the steps started in `[from, to)`, agent task steps
    /// attributed to their agent.
    fn spend(
        &mut self,
        organization_id: OrganizationId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> impl Future<Output = Result<Spend, CoreError>> + Send;
}
