use capitainerie_common::{ApprovalId, CoreError, OrganizationId, RunId};
use capitainerie_events::approval::{ApprovalDecided, ApprovalRequested, ApprovalResumed};
use chrono::{DateTime, Utc};

use super::{
    Approval, ApprovalKind, ApprovalStatus, ToolGrant, Verdict, ports::ApprovalRepository,
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    event::ports::EventEmitter,
};

#[derive(Debug, Clone)]
pub struct ApprovalList {
    pub items: Vec<Approval>,
    /// Pending approvals of the organization, whatever the filter.
    pub pending: i64,
}

pub struct ApprovalService<'a, R, E, A> {
    approvals: R,
    emitter: &'a E,
    authorizer: &'a A,
}

impl<'a, R, E, A> ApprovalService<'a, R, E, A>
where
    R: ApprovalRepository,
    E: EventEmitter,
    A: Authorizer,
{
    pub fn new(approvals: R, emitter: &'a E, authorizer: &'a A) -> Self {
        Self {
            approvals,
            emitter,
            authorizer,
        }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        status: Option<ApprovalStatus>,
    ) -> Result<ApprovalList, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ApprovalRead, None).await?;
        let items = self.approvals.list(organization_id, status).await?;
        let pending = self.approvals.count_pending(organization_id).await?;
        Ok(ApprovalList { items, pending })
    }

    pub async fn get(mut self, subject: Subject, id: ApprovalId) -> Result<Approval, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ApprovalRead, Some(id))
            .await?;
        self.approvals
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    pub async fn decide(
        mut self,
        subject: Subject,
        id: ApprovalId,
        verdict: Verdict,
        note: Option<String>,
    ) -> Result<Approval, CoreError> {
        let organization_id = subject.organization_id;
        let by = subject.id.clone();
        self.require(subject, Action::ApprovalDecide, Some(id))
            .await?;
        let mut approval = self
            .approvals
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        approval.decide(verdict, by.clone(), note, Utc::now())?;
        if !self.approvals.save_decision(&approval).await? {
            return Err(CoreError::Conflict(
                "the approval was decided meanwhile".into(),
            ));
        }
        self.emitter.emit(&ApprovalDecided {
            approval_id: approval.id.0,
            organization_id: organization_id.0,
            status: approval.status.as_str().to_owned(),
            decided_by: by,
            card_id: approval.card_id.map(|id| id.0),
            run_id: approval.run_id.map(|id| id.0),
        })?;
        Ok(approval)
    }

    /// Files a request on behalf of the system, which is not a member: no
    /// policy applies, the caller decides when an approval is needed. A tool
    /// call already pending for the same card with the same arguments is
    /// returned instead of filing it twice.
    pub async fn request(mut self, approval: Approval) -> Result<Approval, CoreError> {
        if let (ApprovalKind::ToolCall, Some(card_id), Some(tool), Some(hash)) = (
            approval.kind,
            approval.card_id,
            approval.tool(),
            approval.arguments_hash.as_deref(),
        ) && let Some(pending) = self
            .approvals
            .find_pending_tool_call(approval.organization_id, card_id, tool, hash)
            .await?
        {
            return Ok(pending);
        }
        self.approvals.insert(&approval).await?;
        self.emitter.emit(&ApprovalRequested {
            approval_id: approval.id.0,
            organization_id: approval.organization_id.0,
            kind: approval.kind.as_str().to_owned(),
            requested_by_agent_id: approval.requested_by_agent_id.map(|id| id.0),
            card_id: approval.card_id.map(|id| id.0),
            run_id: approval.run_id.map(|id| id.0),
        })?;
        Ok(approval)
    }

    /// Hands an approved tool call to the run started to carry it out: the
    /// run may make that call once, and the approval points to the run.
    pub async fn link_resumed(
        mut self,
        approval: &Approval,
        run_id: RunId,
        initiated_by: String,
        now: DateTime<Utc>,
    ) -> Result<ToolGrant, CoreError> {
        let grant = approval.grant(run_id, now)?;
        if !self
            .approvals
            .mark_resumed(approval.organization_id, approval.id, run_id)
            .await?
        {
            return Err(CoreError::Conflict(
                "the approval was resumed already".into(),
            ));
        }
        self.approvals.insert_grant(&grant).await?;
        self.emitter.emit(&ApprovalResumed {
            approval_id: approval.id.0,
            organization_id: approval.organization_id.0,
            card_id: approval.card_id.map(|id| id.0),
            run_id: approval.run_id.map(|id| id.0),
            resumed_run_id: run_id.0,
            decided_by: approval.decided_by.clone().unwrap_or_default(),
            initiated_by,
            tool: grant.tool.clone(),
        })?;
        Ok(grant)
    }

    /// Spends the grant of a run for this call; false when the run has no
    /// grant covering it or spent it already.
    pub async fn use_grant(
        mut self,
        organization_id: OrganizationId,
        grant: &ToolGrant,
        tool: &str,
        arguments: &serde_json::Value,
        now: DateTime<Utc>,
    ) -> Result<bool, CoreError> {
        if !grant.covers(tool, arguments) {
            return Ok(false);
        }
        self.approvals
            .use_grant(organization_id, grant.approval_id, grant.run_id, now)
            .await
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        id: Option<ApprovalId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, action, ResourceKind::Approval, id.map(|id| id.0)),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use capitainerie_common::{AgentId, CardId, OrganizationId, RunId};
    use capitainerie_events::DomainEvent;
    use serde_json::json;

    use super::*;
    use crate::domain::{
        approval::ports::MockApprovalRepository,
        authorization::{Decision, ports::MockAuthorizer},
    };

    #[derive(Default)]
    struct RecordingEmitter(Mutex<Vec<String>>);

    impl EventEmitter for RecordingEmitter {
        fn emit<Ev: DomainEvent>(&self, _event: &Ev) -> Result<(), CoreError> {
            self.0.lock().unwrap().push(Ev::KIND.to_owned());
            Ok(())
        }
    }

    fn human(organization_id: OrganizationId) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id,
            actor: None,
            is_admin: false,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    fn pending(organization_id: OrganizationId) -> Approval {
        Approval::tool_call(
            organization_id,
            (AgentId::new(), "Dev"),
            CardId::new(),
            RunId::new(),
            "github__merge",
            &json!({}),
            Utc::now(),
        )
    }

    fn holding(approval: Approval) -> MockApprovalRepository {
        let mut repository = MockApprovalRepository::new();
        repository.expect_find().returning(move |_, _| {
            let found = approval.clone();
            Box::pin(async move { Ok(Some(found)) })
        });
        repository
    }

    #[tokio::test]
    async fn a_decision_is_saved_and_announced() {
        let organization_id = OrganizationId::new();
        let approval = pending(organization_id);
        let mut repository = holding(approval.clone());
        repository
            .expect_save_decision()
            .times(1)
            .withf(|saved| {
                saved.status == ApprovalStatus::Approved
                    && saved.decided_by.as_deref() == Some("user-1")
            })
            .returning(|_| Box::pin(async { Ok(true) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();

        let decided = ApprovalService::new(repository, &emitter, &authorizer)
            .decide(
                human(organization_id),
                approval.id,
                Verdict::Approve,
                Some("ok".into()),
            )
            .await
            .unwrap();

        assert_eq!(decided.status, ApprovalStatus::Approved);
        assert_eq!(decided.decision_note.as_deref(), Some("ok"));
        assert_eq!(*emitter.0.lock().unwrap(), ["approval.decided"]);
    }

    #[tokio::test]
    async fn a_lost_race_is_a_conflict_and_announces_nothing() {
        let organization_id = OrganizationId::new();
        let approval = pending(organization_id);
        let mut repository = holding(approval.clone());
        repository
            .expect_save_decision()
            .returning(|_| Box::pin(async { Ok(false) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();
        let error = ApprovalService::new(repository, &emitter, &authorizer)
            .decide(human(organization_id), approval.id, Verdict::Reject, None)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Conflict(_)));
        assert!(emitter.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_request_is_stored_and_announced_without_a_policy() {
        let mut repository = MockApprovalRepository::new();
        repository
            .expect_find_pending_tool_call()
            .returning(|_, _, _, _| Box::pin(async { Ok(None) }));
        repository
            .expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let emitter = RecordingEmitter::default();
        let authorizer = MockAuthorizer::new();
        ApprovalService::new(repository, &emitter, &authorizer)
            .request(pending(OrganizationId::new()))
            .await
            .unwrap();
        assert_eq!(*emitter.0.lock().unwrap(), ["approval.requested"]);
    }

    #[tokio::test]
    async fn a_tool_call_already_pending_is_reused_and_not_announced_again() {
        let organization_id = OrganizationId::new();
        let existing = pending(organization_id);
        let mut repeated = existing.clone();
        repeated.id = ApprovalId::new();
        repeated.run_id = Some(RunId::new());
        let mut repository = MockApprovalRepository::new();
        let found = existing.clone();
        let tool = existing.tool().unwrap().to_owned();
        let hash = existing.arguments_hash.clone().unwrap();
        let card_id = existing.card_id.unwrap();
        repository
            .expect_find_pending_tool_call()
            .times(1)
            .withf(move |_, card, asked_tool, asked_hash| {
                *card == card_id && asked_tool == tool && asked_hash == hash
            })
            .returning(move |_, _, _, _| {
                let found = found.clone();
                Box::pin(async move { Ok(Some(found)) })
            });
        repository.expect_insert().never();
        let emitter = RecordingEmitter::default();
        let authorizer = MockAuthorizer::new();
        let filed = ApprovalService::new(repository, &emitter, &authorizer)
            .request(repeated)
            .await
            .unwrap();
        assert_eq!(filed.id, existing.id);
        assert!(emitter.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_approved_call_is_linked_to_its_run_with_a_grant_and_announced() {
        let organization_id = OrganizationId::new();
        let mut approval = pending(organization_id);
        approval
            .decide(Verdict::Approve, "user-2".into(), None, Utc::now())
            .unwrap();
        let run_id = RunId::new();
        let mut repository = MockApprovalRepository::new();
        let id = approval.id;
        repository
            .expect_mark_resumed()
            .times(1)
            .withf(move |_, approval_id, run| *approval_id == id && *run == run_id)
            .returning(|_, _, _| Box::pin(async { Ok(true) }));
        repository
            .expect_insert_grant()
            .times(1)
            .withf(move |grant| grant.run_id == run_id && grant.tool == "github__merge")
            .returning(|_| Box::pin(async { Ok(()) }));
        let emitter = RecordingEmitter::default();
        let authorizer = MockAuthorizer::new();
        let grant = ApprovalService::new(repository, &emitter, &authorizer)
            .link_resumed(&approval, run_id, "user-1".into(), Utc::now())
            .await
            .unwrap();
        assert_eq!(grant.approval_id, approval.id);
        assert_eq!(*emitter.0.lock().unwrap(), ["approval.resumed"]);
    }

    #[tokio::test]
    async fn a_grant_is_spent_only_for_its_call() {
        let organization_id = OrganizationId::new();
        let mut approval = pending(organization_id);
        approval
            .decide(Verdict::Approve, "user-2".into(), None, Utc::now())
            .unwrap();
        let grant = approval.grant(RunId::new(), Utc::now()).unwrap();
        let mut repository = MockApprovalRepository::new();
        repository
            .expect_use_grant()
            .times(1)
            .returning(|_, _, _, _| Box::pin(async { Ok(true) }));
        let emitter = RecordingEmitter::default();
        let authorizer = MockAuthorizer::new();
        let other = ApprovalService::new(MockApprovalRepository::new(), &emitter, &authorizer)
            .use_grant(
                organization_id,
                &grant,
                "github__merge",
                &json!({"x": 1}),
                Utc::now(),
            )
            .await
            .unwrap();
        assert!(!other);
        let same = ApprovalService::new(repository, &emitter, &authorizer)
            .use_grant(
                organization_id,
                &grant,
                "github__merge",
                &json!({}),
                Utc::now(),
            )
            .await
            .unwrap();
        assert!(same);
    }

    #[tokio::test]
    async fn the_list_carries_the_pending_count() {
        let organization_id = OrganizationId::new();
        let mut repository = MockApprovalRepository::new();
        let items = vec![pending(organization_id)];
        repository
            .expect_list()
            .withf(|_, status| *status == Some(ApprovalStatus::Approved))
            .returning(move |_, _| {
                let items = items.clone();
                Box::pin(async move { Ok(items) })
            });
        repository
            .expect_count_pending()
            .returning(|_| Box::pin(async { Ok(4) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();
        let list = ApprovalService::new(repository, &emitter, &authorizer)
            .list(human(organization_id), Some(ApprovalStatus::Approved))
            .await
            .unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.pending, 4);
    }

    #[tokio::test]
    async fn a_denied_member_decides_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "humans only".into(),
                })
            })
        });
        let emitter = RecordingEmitter::default();
        let error = ApprovalService::new(MockApprovalRepository::new(), &emitter, &authorizer)
            .decide(
                human(OrganizationId::new()),
                ApprovalId::new(),
                Verdict::Approve,
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
