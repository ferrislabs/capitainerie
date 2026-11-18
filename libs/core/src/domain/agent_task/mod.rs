pub mod ports;
pub mod tools;

use std::time::Duration;

use capitainerie_common::{AgentId, CardId, CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};

use crate::domain::{conversation::turn::TurnLimits, provider::ModelRef};

pub const AGENT_ACTOR_PREFIX: &str = "agent:";
pub const AGENT_TASK_MAX_TOOL_CALLS: usize = 30;
pub const AGENT_TASK_MAX_OUTPUT_TOKENS: u32 = 16_000;
pub const AGENT_TASK_DEADLINE: Duration = Duration::from_secs(10 * 60);

pub fn agent_task_limits() -> TurnLimits {
    TurnLimits {
        max_tool_calls: AGENT_TASK_MAX_TOOL_CALLS,
        max_output_tokens: AGENT_TASK_MAX_OUTPUT_TOKENS,
    }
}

pub fn actor_of(slug: &str) -> String {
    format!("{AGENT_ACTOR_PREFIX}{slug}")
}

pub fn is_agent_actor(actor: &str) -> bool {
    actor.starts_with(AGENT_ACTOR_PREFIX)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTask {
    pub run_id: RunId,
    pub organization_id: OrganizationId,
    pub card_id: CardId,
    pub agent_id: AgentId,
    pub allow_writes: bool,
    /// The provider and model the run resolved to, kept so a resumed run
    /// keeps them. Never the key.
    pub model: Option<ModelRef>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartAgentRun {
    pub expected_version: i32,
    pub allow_writes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentRunRefusal {
    #[error("la carte n'est assignée à aucun agent")]
    NoAssignee,
    #[error("l'agent assigné est archivé")]
    AgentArchived,
    #[error("l'agent n'a aucun connecteur actif")]
    NoConnector,
    #[error("le connecteur {0} n'est pas connecté")]
    NotConnected(String),
    #[error("{}", crate::domain::user_provider::NO_PROVIDER_MESSAGE)]
    NoProvider,
}

impl AgentRunRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoAssignee => "card_without_agent",
            Self::AgentArchived => "agent_archived",
            Self::NoConnector => "agent_without_connector",
            Self::NotConnected(_) => "not_connected",
            Self::NoProvider => "no_provider",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentRunError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Refused(#[from] AgentRunRefusal),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_agent_actor_is_the_slug_behind_a_prefix() {
        assert_eq!(actor_of("darnold"), "agent:darnold");
        assert!(is_agent_actor("agent:darnold"));
        assert!(!is_agent_actor("capitainerie-agent-writer"));
    }

    #[test]
    fn refusals_have_stable_codes() {
        assert_eq!(AgentRunRefusal::NoAssignee.code(), "card_without_agent");
        assert_eq!(AgentRunRefusal::AgentArchived.code(), "agent_archived");
        assert_eq!(
            AgentRunRefusal::NoConnector.code(),
            "agent_without_connector"
        );
        let refusal = AgentRunRefusal::NotConnected("mestier".into());
        assert_eq!(refusal.code(), "not_connected");
        assert!(refusal.to_string().contains("mestier"));
        assert_eq!(AgentRunRefusal::NoProvider.code(), "no_provider");
        assert!(
            AgentRunRefusal::NoProvider
                .to_string()
                .starts_with("Aucun fournisseur LLM connecté")
        );
    }
}
