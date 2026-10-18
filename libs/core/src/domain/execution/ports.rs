use capitainerie_common::{CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use super::{
    DraftPullRequest, GithubInstallation, PullRequestUrl, RepositoryRef, TaskGrant, TaskLine,
    TaskOutcome, TaskSpec,
};

/// Runs one execution in a disposable container. Implementations must not
/// leak their client types through this port.
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait TaskRunner: Send + Sync {
    /// Starts the task. Starting a task that already exists for this run is
    /// not an error: a resumed run attaches to it.
    fn start(&self, spec: &TaskSpec) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Sends every output line to `lines` until the task ends, then returns
    /// how it ended.
    fn follow(
        &self,
        run_id: RunId,
        lines: mpsc::Sender<TaskLine>,
    ) -> impl Future<Output = Result<TaskOutcome, CoreError>> + Send;

    fn delete(&self, run_id: RunId) -> impl Future<Output = Result<(), CoreError>> + Send;
}

/// The code host the organization connected (GitHub).
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait CodeHost: Send + Sync {
    fn repositories(
        &self,
        installation_id: i64,
    ) -> impl Future<Output = Result<Vec<RepositoryRef>, CoreError>> + Send;

    fn open_draft_pr(
        &self,
        installation_id: i64,
        pull_request: DraftPullRequest,
    ) -> impl Future<Output = Result<PullRequestUrl, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait TaskGrantRepository: Send {
    /// Stores the grant under the SHA-256 of the token.
    fn insert(
        &mut self,
        token_sha256: [u8; 32],
        grant: &TaskGrant,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        token_sha256: [u8; 32],
    ) -> impl Future<Output = Result<Option<TaskGrant>, CoreError>> + Send;

    /// Revokes every grant of the run; revoking twice is not an error.
    fn revoke(
        &mut self,
        run_id: RunId,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait GithubInstallationRepository: Send {
    fn get(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<Option<GithubInstallation>, CoreError>> + Send;

    /// One installation per organization: saving replaces the previous one.
    fn save(
        &mut self,
        installation: &GithubInstallation,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn delete(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
