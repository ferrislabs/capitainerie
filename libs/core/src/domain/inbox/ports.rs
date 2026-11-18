use capitainerie_common::{CoreError, InboxItemId, OrganizationId};
use capitainerie_events::EventEnvelope;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{InboxItem, ReadSelection};

/// A run as a notification describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFacts {
    pub initiated_by: String,
    /// The agent's name, for an agent task run.
    pub agent_name: Option<String>,
    pub card_id: Option<Uuid>,
    pub card_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardFacts {
    pub created_by: String,
    pub title: String,
}

/// A source or a routine: who made it, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedFacts {
    pub created_by: String,
    pub name: String,
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait InboxRepository: Send {
    /// Newest first.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        member: &str,
        unread_only: bool,
        before: Option<InboxItemId>,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<InboxItem>, CoreError>> + Send;

    /// `(total, unread)` of the member.
    fn counts(
        &mut self,
        organization_id: OrganizationId,
        member: &str,
    ) -> impl Future<Output = Result<(i64, i64), CoreError>> + Send;

    /// Marks the member's unread items of the selection; returns how many changed.
    fn mark_read(
        &mut self,
        organization_id: OrganizationId,
        member: &str,
        selection: &ReadSelection,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<u64, CoreError>> + Send;
}

/// What the projection reads and writes. One instance lives in one
/// transaction, which holds the projection's cursor.
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait InboxFeed: Send {
    /// Locks the cursor of the projection and returns the last event it read.
    fn lock_cursor(&mut self, name: &str) -> impl Future<Output = Result<Uuid, CoreError>> + Send;

    /// Events after `cursor`, oldest first, that are at least `settle_secs` old.
    fn events_after(
        &mut self,
        cursor: Uuid,
        settle_secs: i64,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<EventEnvelope>, CoreError>> + Send;

    fn advance(
        &mut self,
        name: &str,
        to: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// False when the member already has an item for that event.
    fn insert(&mut self, item: &InboxItem) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn run(
        &mut self,
        organization_id: OrganizationId,
        run_id: Uuid,
    ) -> impl Future<Output = Result<Option<RunFacts>, CoreError>> + Send;

    fn card(
        &mut self,
        organization_id: OrganizationId,
        card_id: Uuid,
    ) -> impl Future<Output = Result<Option<CardFacts>, CoreError>> + Send;

    fn source(
        &mut self,
        organization_id: OrganizationId,
        source_id: Uuid,
    ) -> impl Future<Output = Result<Option<OwnedFacts>, CoreError>> + Send;

    fn routine(
        &mut self,
        organization_id: OrganizationId,
        routine_id: Uuid,
    ) -> impl Future<Output = Result<Option<OwnedFacts>, CoreError>> + Send;

    fn approval_title(
        &mut self,
        organization_id: OrganizationId,
        approval_id: Uuid,
    ) -> impl Future<Output = Result<Option<String>, CoreError>> + Send;

    /// The name of a crew agent from its actor (`agent:<slug>`).
    fn agent_name(
        &mut self,
        organization_id: OrganizationId,
        slug: &str,
    ) -> impl Future<Output = Result<Option<String>, CoreError>> + Send;
}
