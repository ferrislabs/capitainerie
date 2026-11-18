use capitainerie_common::{CoreError, OrganizationId, RoutineId, RoutineTriggerId};
use chrono::{DateTime, Utc};

use super::{Routine, RoutineListing, RoutineRun, RoutineRunView, Trigger};
use crate::domain::source::TokenHash;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait RoutineRepository: Send {
    fn insert(&mut self, routine: &Routine) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn update(&mut self, routine: &Routine) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: RoutineId,
    ) -> impl Future<Output = Result<Option<RoutineListing>, CoreError>> + Send;

    /// By name; `archived` of `None` lists every routine.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> impl Future<Output = Result<Vec<RoutineListing>, CoreError>> + Send;

    fn insert_trigger(
        &mut self,
        trigger: &Trigger,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn update_trigger(
        &mut self,
        trigger: &Trigger,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Whether a trigger was removed.
    fn delete_trigger(
        &mut self,
        organization_id: OrganizationId,
        routine_id: RoutineId,
        id: RoutineTriggerId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// Across organizations: the token alone designates the trigger. Locks it.
    fn find_by_token(
        &mut self,
        hash: &TokenHash,
    ) -> impl Future<Output = Result<Option<(Routine, Trigger)>, CoreError>> + Send;

    /// Locks, skipping those another worker holds, up to `limit` enabled
    /// triggers of active routines that are due at `now` or hold a queued
    /// passage.
    fn claim_due(
        &mut self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<(Routine, Trigger)>, CoreError>> + Send;

    fn mark_ran(
        &mut self,
        organization_id: OrganizationId,
        id: RoutineId,
        at: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait RoutineRunRepository: Send {
    fn insert(&mut self, run: &RoutineRun) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Saves the status, card, run and reason.
    fn save(&mut self, run: &RoutineRun) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn queued(
        &mut self,
        trigger_id: RoutineTriggerId,
    ) -> impl Future<Output = Result<Option<RoutineRun>, CoreError>> + Send;

    /// Whether the routine's last card is neither done nor rejected.
    fn previous_card_open(
        &mut self,
        routine_id: RoutineId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// Newest first, with the total count.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        routine_id: RoutineId,
        limit: i64,
        offset: i64,
    ) -> impl Future<Output = Result<(Vec<RoutineRunView>, i64), CoreError>> + Send;

    /// Fails the passages still `pending` since before `before`; returns how many.
    fn fail_stale(
        &mut self,
        before: DateTime<Utc>,
        reason: &str,
    ) -> impl Future<Output = Result<u64, CoreError>> + Send;
}
