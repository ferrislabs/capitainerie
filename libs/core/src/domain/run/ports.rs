use capitainerie_common::{CoreError, OrganizationId, RunId};

use super::{Run, RunOverview};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait RunRepository: Send {
    fn insert(&mut self, run: &Run) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Newest first, with the total count of the organization's runs.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        limit: u32,
        offset: u32,
    ) -> impl Future<Output = Result<(Vec<RunOverview>, u64), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: RunId,
    ) -> impl Future<Output = Result<Option<RunOverview>, CoreError>> + Send;
}
