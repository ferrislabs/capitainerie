//! Starts the card's agent when a person hands it work from the console:
//! creating a card for an agent, assigning one, or replying on its card.

use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_core::domain::{
    agent_task::{RepositoryTarget, StartAgentRun},
    authorization::Subject,
    card::Column,
    crew::CrewAgent,
    execution::RepositoryRef,
    run::RunSubject,
};
use capitainerie_github::GithubApps;
use capitainerie_handlers::{ApiError, AppState};
use serde::Serialize;
use utoipa::ToSchema;

use crate::run_agent::{RunAgentError, start_on_repository};

/// What handed the agent work.
pub enum Handoff {
    /// A new card; `repository` picks one when the agent has several.
    Created {
        repository: Option<RepositoryRef>,
    },
    Assigned,
    Replied,
}

/// Whether the agent started. One of `run_id` and `skipped` is set.
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentRunOutcome {
    pub run_id: Option<RunId>,
    /// Why it did not: a refusal code of the agent-runs route (`no_provider`,
    /// `not_connected`…), `agent_busy` (a run is going; a reply posted now
    /// starts the next one when it ends), `choose_repository` (the agent has
    /// several repositories and none was named), or `failed`.
    pub skipped: Option<String>,
    /// French, for the person.
    pub message: Option<String>,
}

impl AgentRunOutcome {
    fn started(run_id: RunId) -> Self {
        Self {
            run_id: Some(run_id),
            skipped: None,
            message: None,
        }
    }

    fn skipped(code: &str, message: impl Into<String>) -> Self {
        Self {
            run_id: None,
            skipped: Some(code.to_owned()),
            message: Some(message.into()),
        }
    }
}

enum Plan {
    Connectors,
    Repository(RepositoryTarget),
}

/// Starts the agent of `card_id` for `subject`, writes off: each write asks
/// for an approval. `None` when there is no agent to start. Never fails the
/// request that handed the work: a refusal comes back as `skipped`.
pub async fn hand_off(
    state: &AppState,
    github: &GithubApps,
    subject: Subject,
    card_id: CardId,
    handoff: Handoff,
) -> Option<AgentRunOutcome> {
    match try_hand_off(state, github, subject, card_id, handoff).await {
        Ok(outcome) => outcome,
        Err(error) => {
            tracing::warn!(%card_id, ?error, "the agent was not started");
            Some(AgentRunOutcome::skipped(
                "failed",
                "L'agent n'a pas pu être lancé.",
            ))
        }
    }
}

async fn try_hand_off(
    state: &AppState,
    github: &GithubApps,
    subject: Subject,
    card_id: CardId,
    handoff: Handoff,
) -> Result<Option<AgentRunOutcome>, CoreError> {
    let detail = state.usecase.card_detail(subject.clone(), card_id).await?;
    let card = detail.card;
    let Some(agent_id) = card.attributes.assignee_agent_id else {
        return Ok(None);
    };
    let open = match handoff {
        Handoff::Replied => !matches!(card.column, Column::Proposed | Column::Rejected),
        Handoff::Created { .. } | Handoff::Assigned => {
            matches!(card.column, Column::Todo | Column::InProgress)
        }
    };
    if !open {
        return Ok(None);
    }
    let agent = state
        .usecase
        .get_crew_agent(subject.clone(), agent_id)
        .await?;
    let previous = match handoff {
        Handoff::Replied => detail
            .runs
            .iter()
            .find_map(|overview| match &overview.run.subject {
                RunSubject::AgentTask { .. } => Some(Plan::Connectors),
                RunSubject::Card {
                    repository,
                    base_branch,
                    ..
                } => Some(Plan::Repository(RepositoryTarget {
                    repository: repository.clone(),
                    base_branch: Some(base_branch.clone()),
                })),
                _ => None,
            }),
        _ => None,
    };
    let requested = match handoff {
        Handoff::Created { repository } => repository,
        _ => None,
    };
    let plan = match previous {
        Some(plan) => plan,
        None => match plan_of(&agent, requested) {
            Ok(plan) => plan,
            Err(outcome) => return Ok(Some(outcome)),
        },
    };
    let command = StartAgentRun {
        expected_version: card.version,
        allow_writes: false,
    };
    let started = match plan {
        Plan::Connectors => state
            .usecase
            .start_agent_run(subject, card_id, command)
            .await
            .map_err(RunAgentError::from),
        Plan::Repository(target) => {
            start_on_repository(state, github, subject, card_id, command, target).await
        }
    };
    Ok(Some(match started {
        Ok(started) => AgentRunOutcome::started(started.run_id),
        Err(RunAgentError::Refused(refusal)) => {
            AgentRunOutcome::skipped(refusal.code(), refusal.to_string())
        }
        Err(RunAgentError::Api(ApiError::Conflict(_))) => AgentRunOutcome::skipped(
            "agent_busy",
            "L'agent travaille déjà sur cette tâche : il lira ce message à la fin de son tour.",
        ),
        Err(RunAgentError::Api(error)) => {
            tracing::warn!(%card_id, ?error, "the agent was not started");
            AgentRunOutcome::skipped("failed", "L'agent n'a pas pu être lancé.")
        }
    }))
}

/// Where the agent works when nothing says otherwise: its connectors, or its
/// only repository; with several, the one the person named.
fn plan_of(agent: &CrewAgent, requested: Option<RepositoryRef>) -> Result<Plan, AgentRunOutcome> {
    let repositories = agent.repository_grants.as_slice();
    if let Some(repository) = requested {
        return Ok(Plan::Repository(RepositoryTarget {
            repository,
            base_branch: None,
        }));
    }
    if !agent.connector_grants.as_slice().is_empty() || repositories.is_empty() {
        return Ok(Plan::Connectors);
    }
    match repositories {
        [only] => Ok(Plan::Repository(RepositoryTarget {
            repository: only.clone(),
            base_branch: None,
        })),
        _ => Err(AgentRunOutcome::skipped(
            "choose_repository",
            "L'agent a accès à plusieurs dépôts : choisis-en un pour le lancer.",
        )),
    }
}
