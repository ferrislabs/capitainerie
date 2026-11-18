pub mod ports;
pub mod tools;

use std::time::Duration;

use capitainerie_common::{AgentId, CardId, CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};

use crate::domain::{
    conversation::turn::TurnLimits,
    execution::{RepositoryRef, valid_branch_name},
    provider::ModelRef,
};

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

/// The repository an agent run works on through the executor, instead of
/// its MCP connectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryTarget {
    pub repository: RepositoryRef,
    /// `None` until the code host told the default branch.
    pub base_branch: Option<String>,
}

impl RepositoryTarget {
    pub fn new(repository: RepositoryRef, base_branch: Option<String>) -> Result<Self, CoreError> {
        let base_branch = base_branch
            .map(|branch| branch.trim().to_owned())
            .filter(|branch| !branch.is_empty());
        if let Some(branch) = &base_branch
            && !valid_branch_name(branch)
        {
            return Err(CoreError::Invalid(format!(
                "base_branch is not a valid branch name: {branch:?}"
            )));
        }
        Ok(Self {
            repository,
            base_branch,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentRunRefusal {
    #[error("la carte n'est assignée à aucun agent")]
    NoAssignee,
    #[error("l'agent assigné est archivé")]
    AgentArchived,
    #[error("l'agent assigné est en pause : reprenez-le avant de le lancer")]
    AgentPaused,
    #[error("l'agent n'a aucun connecteur actif")]
    NoConnector,
    #[error("le connecteur {0} n'est pas connecté")]
    NotConnected(String),
    #[error("{}", crate::domain::user_provider::NO_PROVIDER_MESSAGE)]
    NoProvider,
    #[error("l'agent n'a pas accès au dépôt {0}")]
    RepositoryNotGranted(String),
    #[error("GitHub n'est pas connecté pour cette organisation")]
    GithubNotConnected,
    #[error("les exécutions sur un dépôt sont désactivées : {0}")]
    ExecutorDisabled(String),
}

impl AgentRunRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoAssignee => "card_without_agent",
            Self::AgentArchived => "agent_archived",
            Self::AgentPaused => "agent_paused",
            Self::NoConnector => "agent_without_connector",
            Self::NotConnected(_) => "not_connected",
            Self::NoProvider => "no_provider",
            Self::RepositoryNotGranted(_) => "repository_not_granted",
            Self::GithubNotConnected => "github_not_connected",
            Self::ExecutorDisabled(_) => "executor_disabled",
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
    fn a_repository_target_keeps_a_valid_base_branch_only() {
        let repository: RepositoryRef = "acme/web".parse().unwrap();
        let target = RepositoryTarget::new(repository.clone(), Some(" develop ".into())).unwrap();
        assert_eq!(target.base_branch.as_deref(), Some("develop"));
        let blank = RepositoryTarget::new(repository.clone(), Some("  ".into())).unwrap();
        assert_eq!(blank.base_branch, None);
        assert!(RepositoryTarget::new(repository, Some("--upload-pack=x".into())).is_err());
    }

    #[test]
    fn refusals_have_stable_codes() {
        assert_eq!(AgentRunRefusal::NoAssignee.code(), "card_without_agent");
        assert_eq!(AgentRunRefusal::AgentArchived.code(), "agent_archived");
        assert_eq!(AgentRunRefusal::AgentPaused.code(), "agent_paused");
        assert_eq!(
            AgentRunRefusal::NoConnector.code(),
            "agent_without_connector"
        );
        let refusal = AgentRunRefusal::NotConnected("mestier".into());
        assert_eq!(refusal.code(), "not_connected");
        assert!(refusal.to_string().contains("mestier"));
        assert_eq!(AgentRunRefusal::NoProvider.code(), "no_provider");
        let refusal = AgentRunRefusal::RepositoryNotGranted("acme/web".into());
        assert_eq!(refusal.code(), "repository_not_granted");
        assert!(refusal.to_string().contains("acme/web"));
        assert_eq!(
            AgentRunRefusal::GithubNotConnected.code(),
            "github_not_connected"
        );
        assert_eq!(
            AgentRunRefusal::ExecutorDisabled("x".into()).code(),
            "executor_disabled"
        );
        assert!(
            AgentRunRefusal::NoProvider
                .to_string()
                .starts_with("Aucun fournisseur LLM connecté")
        );
    }
}
