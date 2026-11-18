use capitainerie_common::{ApprovalId, CoreError, OrganizationId};

use super::{Approval, ApprovalStatus};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait ApprovalRepository: Send {
    fn insert(&mut self, approval: &Approval)
    -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ApprovalId,
    ) -> impl Future<Output = Result<Option<Approval>, CoreError>> + Send;

    /// Records the decision of an approval still pending in storage; false
    /// when another decision got there first.
    fn save_decision(
        &mut self,
        approval: &Approval,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// Newest first; `status` of `None` lists every approval.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        status: Option<ApprovalStatus>,
    ) -> impl Future<Output = Result<Vec<Approval>, CoreError>> + Send;

    fn count_pending(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<i64, CoreError>> + Send;
}
