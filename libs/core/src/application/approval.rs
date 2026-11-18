use capitainerie_common::{ApprovalId, CoreError};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    approval::{
        Approval, ApprovalStatus, Verdict,
        service::{ApprovalList, ApprovalService},
    },
    authorization::Subject,
};

impl CapitainerieUseCase {
    #[transactional(approval, emitter, authz)]
    pub async fn list_approvals(
        &self,
        subject: Subject,
        status: Option<ApprovalStatus>,
    ) -> Result<ApprovalList, CoreError> {
        ApprovalService::new(approval_repository, emitter, authz)
            .list(subject, status)
            .await
    }

    #[transactional(approval, emitter, authz)]
    pub async fn get_approval(
        &self,
        subject: Subject,
        id: ApprovalId,
    ) -> Result<Approval, CoreError> {
        ApprovalService::new(approval_repository, emitter, authz)
            .get(subject, id)
            .await
    }

    #[transactional(approval, emitter, authz)]
    pub async fn decide_approval(
        &self,
        subject: Subject,
        id: ApprovalId,
        verdict: Verdict,
        note: Option<String>,
    ) -> Result<Approval, CoreError> {
        ApprovalService::new(approval_repository, emitter, authz)
            .decide(subject, id, verdict, note)
            .await
    }

    #[transactional(approval, emitter, authz)]
    pub async fn request_approval(&self, approval: Approval) -> Result<Approval, CoreError> {
        ApprovalService::new(approval_repository, emitter, authz)
            .request(approval)
            .await
    }
}
