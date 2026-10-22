use capitainerie_common::{CardId, IntegrationActionId, IntegrationInstanceId, RunId};
use capitainerie_core::domain::integration::{
    IntegrationInstance,
    action::Access,
    journal::{IntegrationAction, Outcome},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::instances::dto::{IntegrationEnvironmentDto, IntegrationKindDto};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionAccessDto {
    Read,
    Write,
}

impl From<Access> for ActionAccessDto {
    fn from(access: Access) -> Self {
        match access {
            Access::Read => Self::Read,
            Access::Write => Self::Write,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcomeDto {
    Pending,
    Succeeded,
    Denied,
    Failed,
}

impl From<Outcome> for ActionOutcomeDto {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Pending => Self::Pending,
            Outcome::Succeeded => Self::Succeeded,
            Outcome::Denied => Self::Denied,
            Outcome::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntegrationActionResponse {
    pub id: IntegrationActionId,
    pub run_id: Option<RunId>,
    pub card_id: Option<CardId>,
    pub instance_id: IntegrationInstanceId,
    pub instance_name: String,
    pub instance_environment: IntegrationEnvironmentDto,
    pub instance_kind: IntegrationKindDto,
    pub actor: String,
    pub tool: String,
    pub access: ActionAccessDto,
    #[schema(value_type = Value)]
    pub target: Value,
    #[schema(value_type = Value)]
    pub arguments: Value,
    pub outcome: ActionOutcomeDto,
    pub reason_code: Option<String>,
    pub reason: Option<String>,
    #[schema(value_type = Option<Value>)]
    pub before: Option<Value>,
    #[schema(value_type = Option<Value>)]
    pub after: Option<Value>,
    pub revertible: bool,
    pub reverts_action_id: Option<IntegrationActionId>,
    pub reverted_by_action_id: Option<IntegrationActionId>,
    pub latency_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl IntegrationActionResponse {
    pub fn new(action: IntegrationAction, instance: &IntegrationInstance) -> Self {
        Self {
            id: action.id,
            run_id: action.run_id,
            card_id: action.card_id,
            instance_id: action.instance_id,
            instance_name: instance.name.as_str().to_owned(),
            instance_environment: instance.environment.into(),
            instance_kind: instance.kind.into(),
            actor: action.actor,
            tool: action.tool,
            access: action.access.into(),
            target: action.target,
            arguments: action.arguments,
            outcome: action.outcome.into(),
            reason_code: action.reason_code,
            reason: action.reason,
            before: action.before,
            after: action.after,
            revertible: action.revertible,
            reverts_action_id: action.reverts_action_id,
            reverted_by_action_id: action.reverted_by_action_id,
            latency_ms: action.latency_ms,
            created_at: action.created_at,
            finished_at: action.finished_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntegrationActionListResponse {
    pub items: Vec<IntegrationActionResponse>,
}
