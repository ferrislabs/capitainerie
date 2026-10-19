use capitainerie_common::{CardId, CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};

use super::{Run, RunOverview, step::Step};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait RunRepository: Send {
    fn insert(&mut self, run: &Run) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Newest first, with the total count of the organization's runs.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        limit: u32,
        offset: u32,
    ) -> impl Future<Output = Result<(Vec<RunOverview>, u64), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: RunId,
    ) -> impl Future<Output = Result<Option<RunOverview>, CoreError>> + Send;

    /// Locks the run for the calling worker; `None` if it does not exist or
    /// another worker holds it.
    fn lock(&mut self, id: RunId) -> impl Future<Output = Result<Option<Run>, CoreError>> + Send;

    /// Saves status, timestamps and trace id, and stamps the heartbeat that
    /// tells other workers the run is alive.
    fn save(
        &mut self,
        run: &Run,
        heartbeat_at: Option<DateTime<Utc>>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Runs waiting for a worker: queued ones, and running ones whose worker
    /// stopped beating before `stale_before`. Oldest first, all organizations.
    fn pending(
        &mut self,
        stale_before: DateTime<Utc>,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<RunId>, CoreError>> + Send;

    /// The executions of a card, newest first.
    fn list_for_card(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> impl Future<Output = Result<Vec<RunOverview>, CoreError>> + Send;

    fn has_active_execution(
        &mut self,
        card_id: CardId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn heartbeat_at(
        &mut self,
        id: RunId,
    ) -> impl Future<Output = Result<Option<DateTime<Utc>>, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait StepRepository: Send {
    fn list(&mut self, run_id: RunId) -> impl Future<Output = Result<Vec<Step>, CoreError>> + Send;

    /// Inserts the step, or replaces the one at the same `(run_id, seq)`.
    fn save(&mut self, step: &Step) -> impl Future<Output = Result<(), CoreError>> + Send;
}
