use capitainerie_common::{CoreError, OrganizationId, ProjectId};

use super::{WorksiteIssue, WorksiteSummary};
use crate::domain::execution::RepositoryRef;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait WorksiteRepository: Send {
    /// A second card for the same issue is a conflict.
    fn insert_issue(
        &mut self,
        issue: &WorksiteIssue,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// The issues among `numbers` that already have a card.
    fn issues(
        &mut self,
        organization_id: OrganizationId,
        repository: &RepositoryRef,
        numbers: &[u64],
    ) -> impl Future<Output = Result<Vec<WorksiteIssue>, CoreError>> + Send;

    /// The cards of the project that left « Proposées », newest first.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        project_id: ProjectId,
    ) -> impl Future<Output = Result<Vec<WorksiteSummary>, CoreError>> + Send;
}
