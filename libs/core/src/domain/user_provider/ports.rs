use capitainerie_common::{CoreError, OrganizationId};

use super::{UserProviderCredential, UserProviderSummary};
use crate::domain::provider::ProviderKind;

pub trait UserProviderRepository: Send {
    /// Makes the credential the owner's only default when it is flagged one.
    fn upsert(
        &mut self,
        credential: &UserProviderCredential,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn list(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> impl Future<Output = Result<Vec<UserProviderSummary>, CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
        kind: ProviderKind,
    ) -> impl Future<Output = Result<Option<UserProviderCredential>, CoreError>> + Send;

    fn all(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> impl Future<Output = Result<Vec<UserProviderCredential>, CoreError>> + Send;

    fn delete(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
        kind: ProviderKind,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn set_default(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
        kind: ProviderKind,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
