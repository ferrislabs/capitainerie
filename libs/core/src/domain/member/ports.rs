use capitainerie_common::{CoreError, OrganizationId};

use super::Member;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait MemberRepository: Send {
    /// Inserts the member, or refreshes its name, email and last sighting.
    fn upsert(&mut self, member: &Member) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> impl Future<Output = Result<Option<Member>, CoreError>> + Send;

    /// By name.
    fn list(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Member>, CoreError>> + Send;
}
