use capitainerie_common::{CoreError, OrganizationId, ProjectId};

use super::{AgentRow, Period, ProjectAgentRow, TaskAgentRow, UserModelRow};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait CostRepository: Send {
    /// Spend of the steps started in the period, per project and agent.
    fn project_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
    ) -> impl Future<Output = Result<Vec<ProjectAgentRow>, CoreError>> + Send;

    fn agent_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
    ) -> impl Future<Output = Result<Vec<AgentRow>, CoreError>> + Send;

    fn task_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        project: Option<ProjectId>,
    ) -> impl Future<Output = Result<Vec<TaskAgentRow>, CoreError>> + Send;

    fn user_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
    ) -> impl Future<Output = Result<Vec<UserModelRow>, CoreError>> + Send;

    fn project_exists(
        &mut self,
        organization_id: OrganizationId,
        project_id: ProjectId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
}
