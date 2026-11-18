use capitainerie_common::{ApprovalId, CardId, CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};

use super::{Approval, ApprovalStatus, ToolGrant};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait ApprovalRepository: Send {
    fn insert(&mut self, approval: &Approval)
    -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ApprovalId,
    ) -> impl Future<Output = Result<Option<Approval>, CoreError>> + Send;

    /// The pending request for the same call on the same card, if any.
    fn find_pending_tool_call(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
        tool: &str,
        arguments_hash: &str,
    ) -> impl Future<Output = Result<Option<Approval>, CoreError>> + Send;

    /// Records the decision of an approval still pending in storage; false
    /// when another decision got there first.
    fn save_decision(
        &mut self,
        approval: &Approval,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// Links an approved approval to the run that resumes it; false when it
    /// is linked already.
    fn mark_resumed(
        &mut self,
        organization_id: OrganizationId,
        id: ApprovalId,
        run_id: RunId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn insert_grant(
        &mut self,
        grant: &ToolGrant,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn grant_for_run(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> impl Future<Output = Result<Option<ToolGrant>, CoreError>> + Send;

    /// Spends the grant; false when it was spent already.
    fn use_grant(
        &mut self,
        organization_id: OrganizationId,
        approval_id: ApprovalId,
        run_id: RunId,
        now: DateTime<Utc>,
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
