use capitainerie_common::{ArtifactId, CoreError, OrganizationId};

use super::{Artifact, ArtifactFilter, ArtifactView};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait ArtifactRepository: Send {
    /// False when the run already filed its report.
    fn insert(
        &mut self,
        artifact: &Artifact,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ArtifactId,
    ) -> impl Future<Output = Result<Option<ArtifactView>, CoreError>> + Send;

    /// Newest first, at most `filter.limit`.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &ArtifactFilter,
    ) -> impl Future<Output = Result<Vec<ArtifactView>, CoreError>> + Send;
}
