use capitainerie_common::{ApprovalId, CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::transactional;

use super::{
    CapitainerieUseCase, agent_run::UNNAMED_MEMBER, execute_pipeline::defang_mentions,
    execute_stream::truncate_chars,
};
use crate::domain::{
    agent_task::AgentRunError,
    approval::{
        Approval, ApprovalKind, ApprovalStatus, SYSTEM_ACTOR, Verdict,
        service::{ApprovalList, ApprovalService},
    },
    authorization::Subject,
};

const ALREADY_ACTIVE: &str = "execution_already_active";
const NOTE_IN_COMMENT_CHARS: usize = 1_000;

/// A decision, and for an approved tool call why no run resumed it.
#[derive(Debug, Clone)]
pub struct DecidedApproval {
    pub approval: Approval,
    pub resume_error: Option<String>,
}

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

    /// Records the decision, then acts on it for a tool call on a card: an
    /// approved one starts a new run of the card's agent, for the member who
    /// started the run that asked, allowed to make exactly that call once; a
    /// rejected one is noted on the card. The decision stands whatever
    /// happens next: a run that cannot start is reported in `resume_error`.
    pub async fn decide_approval(
        &self,
        subject: Subject,
        id: ApprovalId,
        verdict: Verdict,
        note: Option<String>,
    ) -> Result<DecidedApproval, CoreError> {
        let mut approval = self.record_decision(subject, id, verdict, note).await?;
        let mut resume_error = None;
        if approval.kind == ApprovalKind::ToolCall
            && let Some(card_id) = approval.card_id
        {
            match approval.status {
                ApprovalStatus::Approved => match self.resume(&approval, card_id).await {
                    Ok(run_id) => approval.resumed_run_id = Some(run_id),
                    Err(reason) => resume_error = Some(reason),
                },
                ApprovalStatus::Rejected => self.note_rejection(&approval, card_id).await,
                ApprovalStatus::Pending | ApprovalStatus::Expired => {}
            }
        }
        Ok(DecidedApproval {
            approval,
            resume_error,
        })
    }

    #[transactional(approval, emitter, authz)]
    async fn record_decision(
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

    async fn resume(&self, approval: &Approval, card_id: CardId) -> Result<RunId, String> {
        let organization_id = approval.organization_id;
        let decider = approval.decided_by.clone().unwrap_or_default();
        let initiator = match approval.run_id {
            Some(run_id) => self
                .run_initiator(organization_id, run_id)
                .await
                .map_err(|error| failure(&error))?,
            None => None,
        };
        let subject = Subject {
            id: initiator.unwrap_or(decider),
            organization_id,
            actor: None,
            is_admin: false,
        };
        match self
            .start_resumed_agent_run(subject, card_id, approval.clone())
            .await
        {
            Ok(started) => Ok(started.run_id),
            Err(AgentRunError::Refused(refusal)) => Err(refusal.to_string()),
            Err(AgentRunError::Core(error)) => {
                tracing::warn!(approval_id = %approval.id.0, %error, "the approved call was not resumed");
                Err(failure(&error))
            }
        }
    }

    async fn run_initiator(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Option<String>, CoreError> {
        sqlx::query_scalar::<_, String>(
            "SELECT initiated_by FROM runs WHERE organization_id = $1 AND id = $2",
        )
        .bind(organization_id.0)
        .bind(run_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| CoreError::Database(error.to_string()))
    }

    /// Posts on the card, as the system for the member who decided, that
    /// the call was refused. A failure is logged: the decision stands.
    async fn note_rejection(&self, approval: &Approval, card_id: CardId) {
        let organization_id = approval.organization_id;
        let decider = approval.decided_by.clone().unwrap_or_default();
        let name = match self.member_name(organization_id, &decider).await {
            Ok(Some(name)) => name,
            Ok(None) => UNNAMED_MEMBER.to_owned(),
            Err(error) => {
                tracing::warn!(approval_id = %approval.id.0, %error, "the decider's name was not found");
                UNNAMED_MEMBER.to_owned()
            }
        };
        let body = rejection_comment(approval, &name);
        let subject = Subject {
            id: decider,
            organization_id,
            actor: Some(SYSTEM_ACTOR.to_owned()),
            is_admin: false,
        };
        if let Err(error) = self.comment_card(subject, card_id, body, None).await {
            tracing::warn!(approval_id = %approval.id.0, %error, "the rejection was not noted on the card");
        }
    }
}

fn failure(error: &CoreError) -> String {
    match error {
        CoreError::Conflict(code) if code == ALREADY_ACTIVE => {
            "une exécution est déjà en cours sur cette tâche : relancez l'agent quand elle sera finie"
                .into()
        }
        CoreError::NotFound => "la tâche est introuvable".into(),
        CoreError::Invalid(message) => message.clone(),
        CoreError::Forbidden { reason } => format!(
            "lancement refusé{}",
            reason
                .as_deref()
                .map(|reason| format!(" : {reason}"))
                .unwrap_or_default()
        ),
        CoreError::Conflict(message) => message.clone(),
        CoreError::Database(_) | CoreError::Internal(_) => {
            "erreur interne, l'exécution n'a pas été lancée".into()
        }
    }
}

fn rejection_comment(approval: &Approval, decider: &str) -> String {
    let tool = approval.tool().unwrap_or("?");
    let mut body = format!(
        "**Demande refusée** : {} a refusé l'appel `{}` demandé par l'agent ; il n'a pas été exécuté et aucune exécution n'est relancée.",
        defang_mentions(decider),
        defang_mentions(tool)
    );
    if let Some(note) = approval.decision_note.as_deref() {
        body.push_str(&format!(
            "\n\nNote : {}",
            defang_mentions(&truncate_chars(note.trim(), NOTE_IN_COMMENT_CHARS))
        ));
    }
    body
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{AgentId, OrganizationId};
    use chrono::Utc;
    use serde_json::json;

    use super::*;

    #[test]
    fn a_rejection_comment_names_the_decider_the_tool_and_the_note() {
        let mut approval = Approval::tool_call(
            OrganizationId::new(),
            (AgentId::new(), "Dev"),
            CardId::new(),
            RunId::new(),
            "mestier__create_order",
            &json!({}),
            Utc::now(),
        );
        approval
            .decide(
                Verdict::Reject,
                "user-1".into(),
                Some("pas ce client".into()),
                Utc::now(),
            )
            .unwrap();
        let body = rejection_comment(&approval, "Alice");
        assert!(
            body.starts_with(
                "**Demande refusée** : Alice a refusé l'appel `mestier__create_order`"
            )
        );
        assert!(body.ends_with("Note : pas ce client"));
    }
}
