use std::time::Duration;

use capitainerie_common::{
    CardId, ClientId, CoreError, IntegrationInstanceId, OrganizationId, RunId,
};
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{action::Access, journal::Grant};
use crate::domain::{
    agent::Usage,
    card::{Card, Column},
    conversation::{
        ToolRecord,
        turn::{ToolSpec, TurnLimits},
    },
    provider::ModelTarget,
};

pub const INTERVENTION_MAX_TOOL_CALLS: usize = 60;
pub const INTERVENTION_MAX_OUTPUT_TOKENS: u32 = 32_000;
pub const INTERVENTION_DEADLINE: Duration = Duration::from_secs(20 * 60);
pub const INSTRUCTIONS_MAX_CHARS: usize = 4_000;
pub const LIST_INSTANCES_TOOL: &str = "list_instances";

pub fn intervention_limits() -> TurnLimits {
    TurnLimits {
        max_tool_calls: INTERVENTION_MAX_TOOL_CALLS,
        max_output_tokens: INTERVENTION_MAX_OUTPUT_TOKENS,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Intervention {
    pub run_id: RunId,
    pub organization_id: OrganizationId,
    pub card_id: CardId,
    pub client_id: ClientId,
    pub instructions: String,
    pub grants: Vec<Grant>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfideCard {
    pub instructions: String,
    pub grants: Vec<(IntegrationInstanceId, Access)>,
    pub confirm_production_writes: bool,
    pub expected_version: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfideRefusal {
    #[error("the card has no client: attach it to a client first")]
    NoClient,
    #[error("give at least one instance to the intervention")]
    NoGrant,
    #[error("this instance belongs to another client than the card")]
    InstanceOfAnotherClient,
    #[error("this instance is archived")]
    InstanceArchived,
    #[error("writing to a production instance needs confirm_production_writes")]
    ProductionWriteNotConfirmed,
    #[error("the instructions exceed 4000 characters")]
    InstructionsTooLong,
}

impl ConfideRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoClient => "card_without_client",
            Self::NoGrant => "no_instance_granted",
            Self::InstanceOfAnotherClient => "instance_of_another_client",
            Self::InstanceArchived => "instance_archived",
            Self::ProductionWriteNotConfirmed => "production_write_not_confirmed",
            Self::InstructionsTooLong => "instructions_too_long",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InterventionError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Refused(#[from] ConfideRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervenableCard {
    card: Card,
    client_id: ClientId,
}

impl IntervenableCard {
    pub fn card(&self) -> &Card {
        &self.card
    }

    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    pub fn into_card(self) -> Card {
        self.card
    }
}

impl TryFrom<Card> for IntervenableCard {
    type Error = InterventionError;

    fn try_from(card: Card) -> Result<Self, Self::Error> {
        if card.column != Column::Todo {
            return Err(CoreError::Conflict("card_not_executable".into()).into());
        }
        let client_id = card
            .links
            .client()
            .map(|client| client.id)
            .ok_or(ConfideRefusal::NoClient)?;
        Ok(Self { card, client_id })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAnswer {
    pub succeeded: bool,
    pub text: String,
    pub summary: String,
}

pub trait IntegratorTools: Send + Sync {
    fn specs(&self) -> Vec<ToolSpec>;

    fn call(&self, name: &str, arguments: &Value) -> impl Future<Output = ToolAnswer> + Send;
}

#[derive(Debug, Clone)]
pub struct IntegratorRequest {
    pub target: ModelTarget,
    pub prompt: String,
    pub limits: TurnLimits,
    pub deadline: Duration,
    pub traceparent: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IntegratorReport {
    pub text: String,
    pub tool_calls: Vec<ToolRecord>,
    pub usage: Usage,
    pub cost_micros: i64,
    pub stopped: Option<String>,
}

pub trait IntegratorRuntime: Send + Sync {
    fn intervene<T: IntegratorTools>(
        &self,
        request: &IntegratorRequest,
        tools: &T,
    ) -> impl Future<Output = Result<IntegratorReport, CoreError>> + Send;
}
