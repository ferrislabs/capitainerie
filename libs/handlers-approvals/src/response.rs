use capitainerie_common::{AgentId, ApprovalId, CardId, RunId};
use capitainerie_core::domain::approval::{Approval, ApprovalKind, ApprovalStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKindDto {
    ToolCall,
    AgentAction,
    RepositoryChange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatusDto {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl From<ApprovalStatus> for ApprovalStatusDto {
    fn from(status: ApprovalStatus) -> Self {
        match status {
            ApprovalStatus::Pending => Self::Pending,
            ApprovalStatus::Approved => Self::Approved,
            ApprovalStatus::Rejected => Self::Rejected,
            ApprovalStatus::Expired => Self::Expired,
        }
    }
}

impl From<ApprovalStatusDto> for ApprovalStatus {
    fn from(status: ApprovalStatusDto) -> Self {
        match status {
            ApprovalStatusDto::Pending => Self::Pending,
            ApprovalStatusDto::Approved => Self::Approved,
            ApprovalStatusDto::Rejected => Self::Rejected,
            ApprovalStatusDto::Expired => Self::Expired,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApprovalResponse {
    pub id: ApprovalId,
    pub kind: ApprovalKindDto,
    pub title: String,
    /// A JSON object. For a `tool_call`, `tool` and `arguments` (secrets
    /// masked). For a `repository_change`, `repository`, `base_branch`, `plan`
    /// (Markdown) and `truncated`.
    #[schema(value_type = Object)]
    pub detail: Value,
    /// The agent asking for the decision; null when the system asks.
    pub requested_by_agent_id: Option<AgentId>,
    /// The card concerned, if any.
    pub card_id: Option<CardId>,
    /// The run that asked, if any.
    pub run_id: Option<RunId>,
    pub status: ApprovalStatusDto,
    /// FerrisKey subject of the member who decided; null while pending.
    pub decided_by: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_note: Option<String>,
    pub created_at: DateTime<Utc>,
    /// The run started when this tool call or plan was approved, to carry it out.
    pub resumed_run_id: Option<RunId>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DecideApprovalResponse {
    #[serde(flatten)]
    pub approval: ApprovalResponse,
    /// Why an approved tool call or plan did not start a new run (no provider,
    /// connector disconnected, agent archived or paused, a run already
    /// active on the card…); null otherwise. The decision stands either way.
    pub resume_error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApprovalListResponse {
    /// Newest first.
    pub items: Vec<ApprovalResponse>,
    /// Pending approvals of the organization, whatever the filter: the badge count.
    pub pending: i64,
}

impl From<Approval> for ApprovalResponse {
    fn from(approval: Approval) -> Self {
        Self {
            id: approval.id,
            kind: match approval.kind {
                ApprovalKind::ToolCall => ApprovalKindDto::ToolCall,
                ApprovalKind::AgentAction => ApprovalKindDto::AgentAction,
                ApprovalKind::RepositoryChange => ApprovalKindDto::RepositoryChange,
            },
            title: approval.title,
            detail: approval.detail,
            requested_by_agent_id: approval.requested_by_agent_id,
            card_id: approval.card_id,
            run_id: approval.run_id,
            status: approval.status.into(),
            decided_by: approval.decided_by,
            decided_at: approval.decided_at,
            decision_note: approval.decision_note,
            created_at: approval.created_at,
            resumed_run_id: approval.resumed_run_id,
        }
    }
}
