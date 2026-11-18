use capitainerie_common::{AgentId, CardId, CoreError, OrganizationId, ProjectId};

use super::{AgentRow, CostFilter, DaySpend, Period, ProjectAgentRow, TaskAgentRow, UserModelRow};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait CostRepository: Send {
    /// Spend of the steps started in the period, per project and agent.
    fn project_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> impl Future<Output = Result<Vec<ProjectAgentRow>, CoreError>> + Send;

    fn agent_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> impl Future<Output = Result<Vec<AgentRow>, CoreError>> + Send;

    fn task_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        project: Option<ProjectId>,
        filter: &CostFilter,
    ) -> impl Future<Output = Result<Vec<TaskAgentRow>, CoreError>> + Send;

    fn user_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> impl Future<Output = Result<Vec<UserModelRow>, CoreError>> + Send;

    /// Spend per UTC day, oldest first; days without a step are absent.
    fn day_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> impl Future<Output = Result<Vec<DaySpend>, CoreError>> + Send;

    fn project_exists(
        &mut self,
        organization_id: OrganizationId,
        project_id: ProjectId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn agent_exists(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn card_exists(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
}
