use capitainerie_common::{CardId, ClientId, CoreError, OrganizationId, ProjectId};
use chrono::{DateTime, Utc};

use super::{Project, ProjectFilter, ProjectListing};
use crate::domain::client::ColumnCount;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait ProjectRepository: Send {
    fn insert(&mut self, project: &Project) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ProjectId,
    ) -> impl Future<Output = Result<Option<Project>, CoreError>> + Send;

    fn update(
        &mut self,
        project: &Project,
    ) -> impl Future<Output = Result<Vec<(CardId, i32)>, CoreError>> + Send;

    fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &ProjectFilter,
    ) -> impl Future<Output = Result<Vec<ProjectListing>, CoreError>> + Send;

    fn of_client(
        &mut self,
        organization_id: OrganizationId,
        client_id: ClientId,
    ) -> impl Future<Output = Result<Vec<Project>, CoreError>> + Send;

    fn card_counts(
        &mut self,
        organization_id: OrganizationId,
        id: ProjectId,
    ) -> impl Future<Output = Result<Vec<ColumnCount>, CoreError>> + Send;

    fn recent(
        &mut self,
        organization_id: OrganizationId,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<(DateTime<Utc>, Project)>, CoreError>> + Send;
}
