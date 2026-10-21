use capitainerie_common::{
    CardId, ClientId, CoreError, IntegrationActionId, IntegrationInstanceId, OrganizationId, RunId,
};
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::action::Access;
use crate::domain::authorization::Subject;

pub const INTEGRATOR_AGENT: &str = "capitainerie-agent-integrator";

pub const SENSITIVE_ODOO_MODELS: [&str; 6] = [
    "ir.config_parameter",
    "ir.mail_server",
    "fetchmail.server",
    "res.users.apikeys",
    "payment.provider",
    "auth.oauth.provider",
];

pub fn is_sensitive_model(model: &str) -> bool {
    SENSITIVE_ODOO_MODELS.contains(&model)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    Succeeded,
    Denied,
    Failed,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

impl TryFrom<&str> for Outcome {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "succeeded" => Ok(Self::Succeeded),
            "denied" => Ok(Self::Denied),
            "failed" => Ok(Self::Failed),
            other => Err(CoreError::Internal(format!(
                "unknown action outcome {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grant {
    pub instance_id: IntegrationInstanceId,
    pub access: Access,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Denial {
    #[error("this instance is not granted to the intervention")]
    InstanceNotGranted,
    #[error("this instance is granted read-only to the intervention")]
    WriteNotGranted,
    #[error("this instance belongs to another client than the card")]
    InstanceOfAnotherClient,
    #[error("this instance is archived")]
    InstanceArchived,
    #[error("the Odoo model {0} holds secrets and is not reachable")]
    SensitiveModel(String),
    #[error("this tool does not apply to this kind of instance")]
    ToolNotForInstance,
    #[error("the policy refused it: {0}")]
    PolicyDenied(String),
    #[error(
        "a masked value has no original to put back: send the real value or leave the field out"
    )]
    MaskedValueWithoutOriginal,
    #[error("not supported by this instance: {0}")]
    NotSupported(String),
}

impl Denial {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InstanceNotGranted => "instance_not_granted",
            Self::WriteNotGranted => "write_not_granted",
            Self::InstanceOfAnotherClient => "instance_of_another_client",
            Self::InstanceArchived => "instance_archived",
            Self::SensitiveModel(_) => "sensitive_model",
            Self::ToolNotForInstance => "tool_not_for_instance",
            Self::PolicyDenied(_) => "policy_denied",
            Self::MaskedValueWithoutOriginal => "masked_value_without_original",
            Self::NotSupported(_) => "not_supported",
        }
    }
}

pub fn ensure_granted(
    grants: &[Grant],
    instance_id: IntegrationInstanceId,
    needed: Access,
) -> Result<(), Denial> {
    let grant = grants
        .iter()
        .find(|grant| grant.instance_id == instance_id)
        .ok_or(Denial::InstanceNotGranted)?;
    if grant.access.allows(needed) {
        Ok(())
    } else {
        Err(Denial::WriteNotGranted)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionContext {
    pub subject: Subject,
    pub run_id: Option<RunId>,
    pub card_id: Option<CardId>,
    pub client_id: ClientId,
    pub grants: Vec<Grant>,
    pub reverts_action_id: Option<IntegrationActionId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    pub succeeded: bool,
    pub text: String,
    pub summary: String,
    pub action_id: Option<IntegrationActionId>,
    pub wrote: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntegrationAction {
    pub id: IntegrationActionId,
    pub organization_id: OrganizationId,
    pub run_id: Option<RunId>,
    pub card_id: Option<CardId>,
    pub instance_id: IntegrationInstanceId,
    pub subject: String,
    pub actor: String,
    pub tool: String,
    pub access: Access,
    pub target: Value,
    pub arguments: Value,
    pub outcome: Outcome,
    pub reason_code: Option<String>,
    pub reason: Option<String>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub revertible: bool,
    pub reverts_action_id: Option<IntegrationActionId>,
    pub reverted_by_action_id: Option<IntegrationActionId>,
    pub latency_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grant_allows_its_level_and_below_only() {
        let instance = IntegrationInstanceId::new();
        let other = IntegrationInstanceId::new();
        let read = [Grant {
            instance_id: instance,
            access: Access::Read,
        }];
        let write = [Grant {
            instance_id: instance,
            access: Access::Write,
        }];
        assert_eq!(ensure_granted(&read, instance, Access::Read), Ok(()));
        assert_eq!(
            ensure_granted(&read, instance, Access::Write),
            Err(Denial::WriteNotGranted)
        );
        assert_eq!(ensure_granted(&write, instance, Access::Write), Ok(()));
        assert_eq!(
            ensure_granted(&write, other, Access::Read),
            Err(Denial::InstanceNotGranted)
        );
    }

    #[test]
    fn sensitive_models_are_listed_exactly() {
        assert!(is_sensitive_model("ir.config_parameter"));
        assert!(!is_sensitive_model("res.partner"));
    }
}
