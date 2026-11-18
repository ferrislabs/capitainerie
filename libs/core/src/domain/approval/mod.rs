pub mod ports;
pub mod service;

use aws_lc_rs::digest;
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
    /// SHA-256 of the canonical JSON of the real (unmasked) tool arguments;
    /// what a grant and the dedupe compare, the masked detail being for display.
    pub arguments_hash: Option<String>,
    /// The run started on approval to carry the call out.
    pub resumed_run_id: Option<RunId>,
}

/// One approved tool call a resumed run may make once, without asking again:
/// the same tool with the same arguments, compared by hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolGrant {
    pub approval_id: ApprovalId,
    pub organization_id: OrganizationId,
    pub run_id: RunId,
    pub tool: String,
    pub arguments_hash: String,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ToolGrant {
    pub fn covers(&self, tool: &str, arguments: &Value) -> bool {
        self.used_at.is_none()
            && self.tool == tool
            && self.arguments_hash == arguments_hash(arguments)
    }
}

/// What a resumed run is told about the call it may now make.
#[derive(Debug, Clone, PartialEq)]
pub struct ApprovedCall {
    pub grant: ToolGrant,
    /// The arguments as the approval shows them, secrets masked.
    pub arguments: Value,
    /// The name of the member who approved.
    pub decided_by: String,
    pub note: Option<String>,
}

/// Author of the comments the system posts on a card about an approval.
pub const SYSTEM_ACTOR: &str = "Capitainerie";

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
            arguments_hash: Some(arguments_hash(arguments)),
            resumed_run_id: None,
        }
    }

    /// The tool a `tool_call` approval asks for.
    pub fn tool(&self) -> Option<&str> {
        match self.kind {
            ApprovalKind::ToolCall => self.detail["tool"].as_str(),
            ApprovalKind::AgentAction => None,
        }
    }

    /// The arguments as shown to members: masked, or a bounded preview.
    pub fn shown_arguments(&self) -> Value {
        match self.detail.get("arguments") {
            Some(arguments) => arguments.clone(),
            None => self.detail["arguments_preview"].clone(),
        }
    }

    /// The one-shot grant an approved tool call gives the run that resumes it.
    pub fn grant(&self, run_id: RunId, now: DateTime<Utc>) -> Result<ToolGrant, CoreError> {
        if self.status != ApprovalStatus::Approved {
            return Err(CoreError::Invalid(format!(
                "the approval is {}, not approved",
                self.status.as_str()
            )));
        }
        let (Some(tool), Some(hash)) = (self.tool(), self.arguments_hash.as_ref()) else {
            return Err(CoreError::Invalid(
                "la demande ne garde pas l'empreinte de l'appel : relancez l'agent avec l'écriture autorisée".into(),
            ));
        };
        Ok(ToolGrant {
            approval_id: self.id,
            organization_id: self.organization_id,
            run_id,
            tool: tool.to_owned(),
            arguments_hash: hash.clone(),
            used_at: None,
            created_at: now,
        })
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

/// Compact JSON with object keys sorted at every level and integral floats
/// written as integers, so equal arguments always hash alike.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                write_canonical(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Number(number) => match number.as_f64() {
            Some(float)
                if !number.is_i64()
                    && !number.is_u64()
                    && float.fract() == 0.0
                    && float.abs() < 9_007_199_254_740_992.0 =>
            {
                #[allow(clippy::cast_possible_truncation)]
                out.push_str(&(float as i64).to_string());
            }
            _ => out.push_str(&number.to_string()),
        },
        other => out.push_str(&other.to_string()),
    }
}

/// Lowercase hex SHA-256 of the canonical JSON of the arguments.
pub fn arguments_hash(arguments: &Value) -> String {
    let hash = digest::digest(&digest::SHA256, canonical_json(arguments).as_bytes());
    hash.as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
    fn the_hash_ignores_key_order_whitespace_and_integral_floats() {
        let one: Value =
            serde_json::from_str(r#"{"b": [1, {"y": 2, "x": 1.0}], "a": "é"}"#).unwrap();
        let two: Value = serde_json::from_str(r#"{"a":"é","b":[1.0,{"x":1,"y":2}]}"#).unwrap();
        assert_eq!(canonical_json(&two), r#"{"a":"é","b":[1,{"x":1,"y":2}]}"#);
        assert_eq!(arguments_hash(&one), arguments_hash(&two));
        assert_eq!(arguments_hash(&one).len(), 64);
        assert_ne!(
            arguments_hash(&json!({"b": [{"x": 1, "y": 2}, 1], "a": "é"})),
            arguments_hash(&one)
        );
        assert_ne!(
            arguments_hash(&json!({"a": 1.5})),
            arguments_hash(&json!({"a": 1}))
        );
        assert_ne!(
            arguments_hash(&json!({"a": "1"})),
            arguments_hash(&json!({"a": 1}))
        );
    }

    #[test]
    fn the_hash_is_of_the_real_arguments_while_the_detail_is_masked() {
        let approval = pending();
        assert_eq!(
            approval.arguments_hash.as_deref(),
            Some(arguments_hash(&json!({"api_key": "sk-123", "amount": 12})).as_str())
        );
        assert_ne!(
            approval.arguments_hash.as_deref(),
            Some(arguments_hash(&approval.detail["arguments"]).as_str())
        );
    }

    #[test]
    fn an_approved_call_grants_exactly_that_call_once() {
        let mut approval = pending();
        let run_id = RunId::new();
        assert!(approval.grant(run_id, Utc::now()).is_err());
        approval
            .decide(Verdict::Approve, "user-1".into(), None, Utc::now())
            .unwrap();
        let mut grant = approval.grant(run_id, Utc::now()).unwrap();
        assert_eq!(grant.tool, "mestier__create_invoice");
        assert_eq!(grant.run_id, run_id);
        assert!(grant.covers(
            "mestier__create_invoice",
            &json!({"api_key": "sk-123", "amount": 12.0})
        ));
        assert!(!grant.covers(
            "mestier__create_invoice",
            &json!({"api_key": "sk-123", "amount": 13})
        ));
        assert!(!grant.covers(
            "mestier__create_invoice",
            &json!({"api_key": "[masqué]", "amount": 12})
        ));
        assert!(!grant.covers(
            "mestier__delete_invoice",
            &json!({"api_key": "sk-123", "amount": 12})
        ));
        grant.used_at = Some(Utc::now());
        assert!(!grant.covers(
            "mestier__create_invoice",
            &json!({"api_key": "sk-123", "amount": 12})
        ));
    }

    #[test]
    fn a_rejected_or_hashless_approval_grants_nothing() {
        let mut rejected = pending();
        rejected
            .decide(Verdict::Reject, "user-1".into(), None, Utc::now())
            .unwrap();
        assert!(rejected.grant(RunId::new(), Utc::now()).is_err());
        let mut legacy = pending();
        legacy.arguments_hash = None;
        legacy
            .decide(Verdict::Approve, "user-1".into(), None, Utc::now())
            .unwrap();
        assert!(legacy.grant(RunId::new(), Utc::now()).is_err());
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
