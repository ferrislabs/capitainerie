pub mod ports;
pub mod service;

use capitainerie_common::{AgentId, ApprovalId, CardId, CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::domain::card::bounded_text;

/// A decision an agent, or the system, asks a member to take.
#[derive(Debug, Clone, PartialEq)]
pub struct Approval {
    pub id: ApprovalId,
    pub organization_id: OrganizationId,
    pub kind: ApprovalKind,
    pub title: String,
    /// A JSON object whose shape depends on the kind.
    pub detail: Value,
    pub requested_by_agent_id: Option<AgentId>,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    pub status: ApprovalStatus,
    pub decided_by: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalKind {
    ToolCall,
    AgentAction,
}

impl ApprovalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolCall => "tool_call",
            Self::AgentAction => "agent_action",
        }
    }
}

impl TryFrom<&str> for ApprovalKind {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "tool_call" => Ok(Self::ToolCall),
            "agent_action" => Ok(Self::AgentAction),
            other => Err(CoreError::Invalid(format!("unknown approval kind {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }
}

impl TryFrom<&str> for ApprovalStatus {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "expired" => Ok(Self::Expired),
            other => Err(CoreError::Invalid(format!(
                "unknown approval status {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Approve,
    Reject,
}

pub const TITLE_MAX_CHARS: usize = 300;
pub const NOTE_MAX_CHARS: usize = 2_000;
pub const DETAIL_MAX_BYTES: usize = 64 * 1024;
/// What a tool call approval keeps of the arguments when they are too large.
const ARGUMENTS_PREVIEW_CHARS: usize = 4_000;

impl Approval {
    /// A pending request for a write tool the run was not allowed to call.
    pub fn tool_call(
        organization_id: OrganizationId,
        agent: (AgentId, &str),
        card_id: CardId,
        run_id: RunId,
        tool: &str,
        arguments: &Value,
        now: DateTime<Utc>,
    ) -> Self {
        let (agent_id, agent_name) = agent;
        let title: String = format!("Écriture demandée par {agent_name} : {tool}")
            .chars()
            .take(TITLE_MAX_CHARS)
            .collect();
        Self {
            id: ApprovalId::new(),
            organization_id,
            kind: ApprovalKind::ToolCall,
            title,
            detail: tool_call_detail(tool, arguments),
            requested_by_agent_id: Some(agent_id),
            card_id: Some(card_id),
            run_id: Some(run_id),
            status: ApprovalStatus::Pending,
            decided_by: None,
            decided_at: None,
            decision_note: None,
            created_at: now,
        }
    }

    pub fn decide(
        &mut self,
        verdict: Verdict,
        by: String,
        note: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<(), CoreError> {
        if self.status != ApprovalStatus::Pending {
            return Err(CoreError::Conflict(format!(
                "the approval is already {}",
                self.status.as_str()
            )));
        }
        let note = match note.map(|note| note.trim().to_owned()) {
            Some(note) if !note.is_empty() => {
                Some(bounded_text(note, NOTE_MAX_CHARS, "note", false)?)
            }
            _ => None,
        };
        self.status = match verdict {
            Verdict::Approve => ApprovalStatus::Approved,
            Verdict::Reject => ApprovalStatus::Rejected,
        };
        self.decided_by = Some(by);
        self.decided_at = Some(now);
        self.decision_note = note;
        Ok(())
    }
}

fn tool_call_detail(tool: &str, arguments: &Value) -> Value {
    let arguments = super::agent_task::tools::masked(arguments);
    let detail = json!({"tool": tool, "arguments": arguments});
    let size = serde_json::to_vec(&detail).map_or(usize::MAX, |bytes| bytes.len());
    if size <= DETAIL_MAX_BYTES {
        return detail;
    }
    let preview: String = arguments
        .to_string()
        .chars()
        .take(ARGUMENTS_PREVIEW_CHARS)
        .collect();
    json!({"tool": tool, "arguments_preview": preview, "truncated": true})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending() -> Approval {
        Approval::tool_call(
            OrganizationId::new(),
            (AgentId::new(), "Dev"),
            CardId::new(),
            RunId::new(),
            "mestier__create_invoice",
            &json!({"amount": 12, "api_key": "sk-123"}),
            Utc::now(),
        )
    }

    #[test]
    fn a_tool_call_request_names_the_tool_and_masks_secrets() {
        let approval = pending();
        assert_eq!(approval.kind, ApprovalKind::ToolCall);
        assert_eq!(approval.status, ApprovalStatus::Pending);
        assert_eq!(
            approval.title,
            "Écriture demandée par Dev : mestier__create_invoice"
        );
        assert_eq!(approval.detail["tool"], "mestier__create_invoice");
        assert_eq!(approval.detail["arguments"]["amount"], 12);
        assert_ne!(approval.detail["arguments"]["api_key"], "sk-123");
    }

    #[test]
    fn huge_arguments_are_kept_as_a_bounded_preview() {
        let approval = Approval::tool_call(
            OrganizationId::new(),
            (AgentId::new(), "Dev"),
            CardId::new(),
            RunId::new(),
            "tool",
            &json!({"blob": "a".repeat(DETAIL_MAX_BYTES)}),
            Utc::now(),
        );
        assert_eq!(approval.detail["truncated"], true);
        assert!(
            approval.detail["arguments_preview"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= ARGUMENTS_PREVIEW_CHARS
        );
    }

    #[test]
    fn a_pending_approval_is_decided_once() {
        let mut approval = pending();
        let now = Utc::now();
        approval
            .decide(
                Verdict::Reject,
                "user-1".into(),
                Some("  non  ".into()),
                now,
            )
            .unwrap();
        assert_eq!(approval.status, ApprovalStatus::Rejected);
        assert_eq!(approval.decided_by.as_deref(), Some("user-1"));
        assert_eq!(approval.decided_at, Some(now));
        assert_eq!(approval.decision_note.as_deref(), Some("non"));
        let again = approval.decide(Verdict::Approve, "user-2".into(), None, now);
        assert!(matches!(again, Err(CoreError::Conflict(_))));
    }

    #[test]
    fn a_blank_note_is_dropped_and_a_long_one_refused() {
        let mut approval = pending();
        approval
            .decide(
                Verdict::Approve,
                "user-1".into(),
                Some(" ".into()),
                Utc::now(),
            )
            .unwrap();
        assert_eq!(approval.decision_note, None);
        let mut approval = pending();
        let error = approval.decide(
            Verdict::Approve,
            "user-1".into(),
            Some("a".repeat(NOTE_MAX_CHARS + 1)),
            Utc::now(),
        );
        assert!(matches!(error, Err(CoreError::Invalid(_))));
        assert_eq!(approval.status, ApprovalStatus::Pending);
    }

    #[test]
    fn statuses_and_kinds_round_trip() {
        for status in [
            ApprovalStatus::Pending,
            ApprovalStatus::Approved,
            ApprovalStatus::Rejected,
            ApprovalStatus::Expired,
        ] {
            assert_eq!(ApprovalStatus::try_from(status.as_str()).unwrap(), status);
        }
        for kind in [ApprovalKind::ToolCall, ApprovalKind::AgentAction] {
            assert_eq!(ApprovalKind::try_from(kind.as_str()).unwrap(), kind);
        }
        assert!(ApprovalStatus::try_from("maybe").is_err());
    }
}
