pub mod contribution;
pub mod harness;
pub mod ports;
pub mod report;

use std::{fmt, time::Duration};

use capitainerie_common::{CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use self::harness::{ANTIGRAVITY_MODELS, CODEX_MODELS, Harness};
use crate::domain::{
    agent::SecretToken,
    card::{Card, Column},
};

/// FerrisKey client of the executor agent.
pub const EXECUTOR_AGENT: &str = "capitainerie-agent-executor";

/// Where an audit writes its report in the task's container: outside the
/// checkout, so the report is never part of the repository.
pub const REPORT_PATH: &str = "/workspace/.capitainerie/report.md";

/// Where an audit writes the change it proposes, when the card asks for one:
/// outside the checkout too.
pub const PLAN_PATH: &str = "/workspace/.capitainerie/plan.md";

/// What an execution may do to its repository.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ExecutionMode {
    /// Changes are pushed to the run's branch and a draft pull request opens.
    #[default]
    Change,
    /// Read-only analysis: nothing is pushed, a report is filed instead.
    Audit,
}

impl ExecutionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Change => "change",
            Self::Audit => "audit",
        }
    }

    pub fn of_writes(allow_writes: bool) -> Self {
        if allow_writes {
            Self::Change
        } else {
            Self::Audit
        }
    }

    pub fn is_read_only(self) -> bool {
        self == Self::Audit
    }
}

impl TryFrom<&str> for ExecutionMode {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "change" => Ok(Self::Change),
            "audit" => Ok(Self::Audit),
            other => Err(CoreError::Invalid(format!(
                "unknown execution mode {other:?}"
            ))),
        }
    }
}

/// A branch name safe on a `git clone --branch` command line and in a URL.
pub fn valid_branch_name(branch: &str) -> bool {
    !branch.is_empty()
        && branch.len() <= 255
        && !branch.starts_with(['-', '/', '.'])
        && !branch.ends_with(['/', '.'])
        && !branch.ends_with(".lock")
        && !branch.contains("..")
        && !branch.contains("//")
        && !branch.contains("@{")
        && branch
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

/// Whether the worker could start executions when it last started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorStatus {
    pub enabled: bool,
    pub reason: Option<String>,
}

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
pub fn valid_segment(segment: &str) -> bool {
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

/// A repository as the code host lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryListing {
    pub repository: RepositoryRef,
    pub private: bool,
    pub html_url: String,
}

/// An open issue of a repository, as the code host lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryIssue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub html_url: String,
    pub labels: Vec<String>,
}

/// The model an execution runs on, checked against the harness that runs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ExecutorModel {
    harness: Harness,
    model: String,
}

impl ExecutorModel {
    pub fn new(harness: Harness, model: impl Into<String>) -> Result<Self, CoreError> {
        let model = model.into();
        if !harness.accepts_model(&model) {
            return Err(CoreError::Invalid(format!(
                "the {harness} executor does not run on {model:?}"
            )));
        }
        Ok(Self { harness, model })
    }

    pub fn harness(&self) -> Harness {
        self.harness
    }

    pub fn as_str(&self) -> &str {
        &self.model
    }
}

/// The harness that accepts the agent's own model, if one does.
pub fn harness_of_agent(agent_model: &str) -> Option<Harness> {
    use crate::domain::provider::model_ref_of;
    Harness::ALL.into_iter().find(|harness| {
        model_ref_of(agent_model, harness.provider()).is_some_and(|wanted| {
            wanted.provider == harness.provider() && harness.accepts_model(&wanted.model)
        })
    })
}

/// The harness an agent's execution runs on: the one that accepts the
/// agent's own model, else the fallback's.
pub fn agent_harness(agent_model: &str, fallback: &ExecutorModel) -> Harness {
    harness_of_agent(agent_model).unwrap_or_else(|| fallback.harness())
}

/// The model an agent's execution runs on: the agent's own model when its
/// harness accepts it, else the default model of the initiator's key for that
/// harness's provider (`user_default`), else `fallback` when it runs on that
/// harness, else the harness's default model.
pub fn agent_executor_model(
    agent_model: &str,
    user_default: Option<&str>,
    fallback: &ExecutorModel,
) -> ExecutorModel {
    use crate::domain::provider::{is_model_id, model_ref_of};
    let harness = agent_harness(agent_model, fallback);
    let provider = harness.provider();
    let agent = model_ref_of(agent_model, provider)
        .filter(|wanted| wanted.provider == provider)
        .map(|wanted| wanted.model);
    let user = user_default
        .map(str::trim)
        .filter(|model| is_model_id(model))
        .map(str::to_owned);
    [agent, user]
        .into_iter()
        .flatten()
        .find_map(|model| ExecutorModel::new(harness, model).ok())
        .or_else(|| (fallback.harness() == harness).then(|| fallback.clone()))
        .unwrap_or_else(|| ExecutorModel {
            harness,
            model: harness.default_model().to_owned(),
        })
}

impl TryFrom<String> for ExecutorModel {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let harness = Harness::of_model(&value).ok_or_else(|| {
            CoreError::Invalid(format!(
                "no executor runs on {value:?}: claude-code runs on claude-* models, codex on {}, antigravity on {}",
                CODEX_MODELS.join(", "),
                ANTIGRAVITY_MODELS.map(|(model, _)| model).join(", ")
            ))
        })?;
        Self::new(harness, value)
    }
}

impl From<ExecutorModel> for String {
    fn from(value: ExecutorModel) -> Self {
        value.model
    }
}

impl Default for ExecutorModel {
    fn default() -> Self {
        let harness = Harness::default();
        Self {
            harness,
            model: harness.default_model().to_owned(),
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubAppCredentials {
    pub organization_id: OrganizationId,
    pub app_id: i64,
    pub slug: String,
    pub client_id: String,
    pub client_secret: SecretToken,
    pub private_key_pem: SecretToken,
    pub webhook_secret: SecretToken,
    pub html_url: String,
    pub owner_login: String,
    pub owner_type: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubAppSummary {
    pub organization_id: OrganizationId,
    pub app_id: i64,
    pub slug: String,
    pub html_url: String,
    pub owner_login: String,
    pub owner_type: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl From<&GithubAppCredentials> for GithubAppSummary {
    fn from(app: &GithubAppCredentials) -> Self {
        Self {
            organization_id: app.organization_id,
            app_id: app.app_id,
            slug: app.slug.clone(),
            html_url: app.html_url.clone(),
            owner_login: app.owner_login.clone(),
            owner_type: app.owner_type.clone(),
            created_by: app.created_by.clone(),
            created_at: app.created_at,
        }
    }
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
    pub mode: ExecutionMode,
    /// Only the credentials proxy accepts it; never a real key.
    pub task_token: SecretToken,
    pub commit: CommitSettings,
}

/// How a change run commits: as whom, what its message may not contain, and
/// the message used when the run's own is missing or leaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSettings {
    pub author: CommitAuthor,
    /// One term per line, matched without case.
    pub leak_terms: String,
    pub fallback_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
}

impl CommitAuthor {
    /// The GitHub App's bot, for a run no member started.
    pub fn app_bot(slug: &str) -> Self {
        let login = format!("{slug}[bot]");
        Self {
            email: format!("{login}@users.noreply.github.com"),
            name: login,
        }
    }
}

/// One line of the task's output.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskLine {
    /// A raw output line of the harness, read by its parser.
    Agent(serde_json::Value),
    /// A marker written by the entrypoint script.
    Capitainerie(TaskMarker),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TaskMarker {
    Cloned,
    Pushed {
        sha: String,
    },
    NoChanges,
    /// A piece of the audit report, base64.
    ReportChunk {
        data: String,
    },
    /// The audit report is complete: the SHA-256 (hex) and size of what the
    /// chunks decode to, and whether the script cut it.
    ReportEnd {
        sha256: String,
        bytes: u64,
        #[serde(default)]
        truncated: bool,
    },
    /// A piece of the plan an audit proposes, base64.
    PlanChunk {
        data: String,
    },
    /// The plan is complete, as `ReportEnd` for the report.
    PlanEnd {
        sha256: String,
        bytes: u64,
        #[serde(default)]
        truncated: bool,
    },
    /// A piece of the title and body a change run wrote, base64.
    ContributionChunk {
        data: String,
    },
    /// The contribution text is complete, as `ReportEnd` for the report.
    ContributionEnd {
        sha256: String,
        bytes: u64,
        #[serde(default)]
        truncated: bool,
    },
    AuditDone,
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
    /// The only model the proxy lets the task ask Anthropic for.
    pub model: String,
    pub mode: ExecutionMode,
    /// Who started the run, whose own Anthropic key is used before the
    /// organization's; `None` on grants issued before it was recorded.
    pub initiated_by: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl TaskGrant {
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && now < self.expires_at
    }
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
    fn the_executor_model_must_suit_its_harness() {
        let harness = Harness::ClaudeCode;
        assert!(ExecutorModel::new(harness, "claude-opus-5").is_ok());
        for refused in ["", "gpt-4o", "llama3", " claude-opus-5", "anthropic/claude"] {
            assert!(
                matches!(
                    ExecutorModel::new(harness, refused),
                    Err(CoreError::Invalid(_))
                ),
                "{refused:?}"
            );
        }
        let Err(CoreError::Invalid(message)) = ExecutorModel::new(harness, "gpt-4o") else {
            panic!("gpt-4o is not a Claude Code model");
        };
        assert!(message.contains("claude-code"), "{message}");
        assert!(serde_json::from_str::<ExecutorModel>("\"gpt-4o\"").is_err());
        assert_eq!(
            serde_json::from_str::<ExecutorModel>("\"claude-haiku-5-5\"")
                .unwrap()
                .as_str(),
            "claude-haiku-5-5"
        );
    }

    #[test]
    fn a_grant_is_usable_until_it_expires_or_is_revoked() {
        let now = Utc::now();
        let mut grant = TaskGrant {
            run_id: RunId::new(),
            organization_id: OrganizationId(uuid::Uuid::nil()),
            repository: "acme/web".parse().unwrap(),
            branch: "capitainerie/0a1b2c3d4e5f".into(),
            model: "claude-sonnet-5-5".into(),
            mode: ExecutionMode::Change,
            initiated_by: None,
            expires_at: now + chrono::Duration::minutes(5),
            revoked_at: None,
        };
        assert!(grant.is_usable(now));
        assert!(!grant.is_usable(now + chrono::Duration::minutes(6)));
        grant.revoked_at = Some(now);
        assert!(!grant.is_usable(now));
    }

    #[test]
    fn the_mode_round_trips_and_only_audit_is_read_only() {
        for mode in [ExecutionMode::Change, ExecutionMode::Audit] {
            assert_eq!(ExecutionMode::try_from(mode.as_str()).unwrap(), mode);
        }
        assert!(ExecutionMode::try_from("push").is_err());
        assert_eq!(ExecutionMode::of_writes(true), ExecutionMode::Change);
        assert_eq!(ExecutionMode::of_writes(false), ExecutionMode::Audit);
        assert!(ExecutionMode::Audit.is_read_only());
        assert!(!ExecutionMode::Change.is_read_only());
        assert_eq!(ExecutionMode::default(), ExecutionMode::Change);
    }

    #[test]
    fn an_agent_runs_on_its_model_then_the_users_default_then_the_fallback() {
        let fallback = ExecutorModel::default();
        let model = |agent: &str, user: Option<&str>| {
            agent_executor_model(agent, user, &fallback)
                .as_str()
                .to_owned()
        };
        assert_eq!(
            model("claude-opus-5", Some("claude-haiku-5-5")),
            "claude-opus-5"
        );
        assert_eq!(model("anthropic:claude-opus-5", None), "claude-opus-5");
        assert_eq!(
            model("mistral:claude-x", None),
            Harness::ClaudeCode.default_model()
        );
        assert_eq!(model("", Some("claude-haiku-5-5")), "claude-haiku-5-5");
        assert_eq!(
            model("gpt-4o", Some("claude-haiku-5-5")),
            "claude-haiku-5-5"
        );
        assert_eq!(
            model("Claude Opus", None),
            Harness::ClaudeCode.default_model()
        );
        assert_eq!(
            model("mistral-large", Some("gemini-3")),
            Harness::ClaudeCode.default_model()
        );
    }

    #[test]
    fn an_agent_on_a_codex_model_runs_on_codex() {
        let fallback = ExecutorModel::default();
        let chosen = agent_executor_model("gpt-6-luna", None, &fallback);
        assert_eq!(chosen.harness(), Harness::Codex);
        assert_eq!(chosen.as_str(), "gpt-6-luna");
        let prefixed = agent_executor_model("openai:gpt-6-sol", Some("gpt-5.5"), &fallback);
        assert_eq!(prefixed.as_str(), "gpt-6-sol");
        assert_eq!(agent_harness("gpt-6-luna", &fallback), Harness::Codex);
        assert_eq!(agent_harness("gpt-4o", &fallback), Harness::ClaudeCode);
        assert_eq!(
            harness_of_agent("mistral-large-4-0"),
            Some(Harness::MistralVibe)
        );
        assert_eq!(harness_of_agent("gpt-4o"), None);
    }

    #[test]
    fn an_agent_without_a_model_keeps_the_fallback() {
        let fallback = ExecutorModel::default();
        assert_eq!(agent_executor_model("", None, &fallback), fallback);
        let codex = ExecutorModel::new(Harness::Codex, "gpt-5.5").unwrap();
        assert_eq!(agent_executor_model("", None, &codex), codex);
        assert_eq!(
            agent_executor_model("claude-opus-5", None, &codex).harness(),
            Harness::ClaudeCode
        );
        assert_eq!(
            agent_executor_model("mistral-large", Some("gpt-6-sol"), &codex).as_str(),
            "gpt-6-sol"
        );
    }

    #[test]
    fn a_base_branch_is_a_plain_ref_name() {
        for good in ["main", "release/2.1", "feat_x-y.z"] {
            assert!(valid_branch_name(good), "{good}");
        }
        for bad in [
            "",
            "-x",
            "/main",
            "a..b",
            "a b",
            "main.lock",
            "a//b",
            "x@{1}",
            "a/",
            ".hidden",
            "a;rm",
            "$(x)",
        ] {
            assert!(!valid_branch_name(bad), "{bad}");
        }
        assert!(!valid_branch_name(&"a".repeat(256)));
    }

    #[test]
    fn the_report_lives_outside_the_checkout() {
        assert!(REPORT_PATH.starts_with("/workspace/"));
        assert!(!REPORT_PATH.starts_with("/workspace/repo/"));
        assert!(PLAN_PATH.starts_with("/workspace/"));
        assert!(!PLAN_PATH.starts_with("/workspace/repo/"));
    }

    #[test]
    fn report_markers_parse_from_the_entrypoint_lines() {
        let chunk: TaskMarker =
            serde_json::from_str(r#"{"type":"capitainerie","event":"report_chunk","data":"aGk="}"#)
                .unwrap();
        assert_eq!(
            chunk,
            TaskMarker::ReportChunk {
                data: "aGk=".into()
            }
        );
        let end: TaskMarker = serde_json::from_str(
            r#"{"type":"capitainerie","event":"report_end","sha256":"ab","bytes":2,"truncated":true}"#,
        )
        .unwrap();
        assert_eq!(
            end,
            TaskMarker::ReportEnd {
                sha256: "ab".into(),
                bytes: 2,
                truncated: true
            }
        );
        let done: TaskMarker =
            serde_json::from_str(r#"{"type":"capitainerie","event":"audit_done"}"#).unwrap();
        assert_eq!(done, TaskMarker::AuditDone);
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
