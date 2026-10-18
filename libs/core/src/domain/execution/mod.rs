pub mod ports;

use std::{fmt, time::Duration};

use capitainerie_common::{CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    agent::SecretToken,
    card::{Card, Column},
};

/// FerrisKey client of the executor agent.
pub const EXECUTOR_AGENT: &str = "capitainerie-agent-executor";

/// A GitHub repository, `owner/name`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RepositoryRef {
    owner: String,
    name: String,
}

impl RepositoryRef {
    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// GitHub logins and repository names: ASCII letters, digits, `-`, `_`, `.`,
/// and never `.` or `..` alone, so the value is safe in a URL path.
fn valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.len() <= 100
        && segment != "."
        && segment != ".."
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl TryFrom<String> for RepositoryRef {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().parse()
    }
}

impl std::str::FromStr for RepositoryRef {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once('/') {
            Some((owner, name)) if valid_segment(owner) && valid_segment(name) => Ok(Self {
                owner: owner.to_owned(),
                name: name.to_owned(),
            }),
            _ => Err(CoreError::Invalid(format!(
                "repository must be owner/name, got {value:?}"
            ))),
        }
    }
}

impl From<RepositoryRef> for String {
    fn from(value: RepositoryRef) -> Self {
        value.to_string()
    }
}

impl fmt::Display for RepositoryRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// The model Claude Code runs on: always an Anthropic model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExecutorModel(String);

impl ExecutorModel {
    pub const DEFAULT: &'static str = "claude-sonnet-5-5";

    pub fn new(model: impl Into<String>) -> Result<Self, CoreError> {
        let model = model.into();
        if model.trim().is_empty() {
            return Err(CoreError::Invalid("executor model is empty".into()));
        }
        Ok(Self(model))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ExecutorModel {
    fn default() -> Self {
        Self(Self::DEFAULT.to_owned())
    }
}

/// A card a human validated, so the only kind an executor may work on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableCard(Card);

impl ExecutableCard {
    pub fn card(&self) -> &Card {
        &self.0
    }

    pub fn into_card(self) -> Card {
        self.0
    }
}

impl TryFrom<Card> for ExecutableCard {
    type Error = CoreError;

    fn try_from(card: Card) -> Result<Self, Self::Error> {
        if card.column == Column::Todo {
            Ok(Self(card))
        } else {
            Err(CoreError::Conflict("card_not_executable".into()))
        }
    }
}

/// The organization's installation of the Capitainerie GitHub App.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubInstallation {
    pub organization_id: OrganizationId,
    pub installation_id: i64,
    pub account_login: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

/// Everything the task runner needs to start one execution.
#[derive(Debug, Clone)]
pub struct TaskSpec {
    pub run_id: RunId,
    pub repository: RepositoryRef,
    pub base_branch: String,
    pub branch: String,
    pub prompt: String,
    pub model: ExecutorModel,
    pub max_turns: u32,
    pub deadline: Duration,
    /// Only the credentials proxy accepts it; never a real key.
    pub task_token: SecretToken,
}

/// One line of the task's output.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskLine {
    /// A Claude Code `stream-json` event.
    Agent(serde_json::Value),
    /// A marker written by the entrypoint script.
    Capitainerie(TaskMarker),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TaskMarker {
    Cloned,
    Pushed { sha: String },
    NoChanges,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    Succeeded,
    Failed { reason: String },
    DeadlineExceeded,
}

/// A pull request the worker opens once a branch was pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftPullRequest {
    pub repository: RepositoryRef,
    pub base: String,
    pub head: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestUrl(pub String);

/// A task token as stored: only its hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGrant {
    pub run_id: RunId,
    pub organization_id: OrganizationId,
    pub repository: RepositoryRef,
    pub branch: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl TaskGrant {
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && now < self.expires_at
    }
}

/// The branch an execution pushes to: `capitainerie/<first 8 hex of the run id>`.
pub fn execution_branch(run_id: RunId) -> String {
    let simple = run_id.0.simple().to_string();
    format!("capitainerie/{}", &simple[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_is_owner_slash_name() {
        let repo: RepositoryRef = "acme/web-app".parse().unwrap();
        assert_eq!(repo.owner(), "acme");
        assert_eq!(repo.name(), "web-app");
        assert_eq!(repo.to_string(), "acme/web-app");
    }

    #[test]
    fn a_repository_refuses_paths_and_empty_parts() {
        for value in [
            "acme",
            "acme/",
            "/web",
            "acme/web/x",
            "acme/..",
            "ac me/web",
            "acme/w?b",
        ] {
            assert!(value.parse::<RepositoryRef>().is_err(), "{value}");
        }
    }

    #[test]
    fn an_execution_branch_uses_the_run_id() {
        let run_id = RunId(uuid::Uuid::parse_str("0a1b2c3d-0000-7000-8000-000000000000").unwrap());
        assert_eq!(execution_branch(run_id), "capitainerie/0a1b2c3d");
    }

    #[test]
    fn a_grant_is_usable_until_it_expires_or_is_revoked() {
        let now = Utc::now();
        let mut grant = TaskGrant {
            run_id: RunId::new(),
            organization_id: OrganizationId(uuid::Uuid::nil()),
            repository: "acme/web".parse().unwrap(),
            branch: "capitainerie/0a1b2c3d".into(),
            expires_at: now + chrono::Duration::minutes(5),
            revoked_at: None,
        };
        assert!(grant.is_usable(now));
        assert!(!grant.is_usable(now + chrono::Duration::minutes(6)));
        grant.revoked_at = Some(now);
        assert!(!grant.is_usable(now));
    }

    #[test]
    fn markers_parse_from_the_entrypoint_lines() {
        let marker: TaskMarker =
            serde_json::from_str(r#"{"event":"pushed","sha":"abc123"}"#).unwrap();
        assert_eq!(
            marker,
            TaskMarker::Pushed {
                sha: "abc123".into()
            }
        );
    }
}
