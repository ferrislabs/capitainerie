use capitainerie_common::{CoreError, OrganizationId, SourceId};
use chrono::{DateTime, Utc};

use super::{Source, SourceEvent, SourceListing, TokenHash};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait SourceRepository: Send {
    fn insert(&mut self, source: &Source) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn update(&mut self, source: &Source) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: SourceId,
    ) -> impl Future<Output = Result<Option<SourceListing>, CoreError>> + Send;

    /// By name; `archived` of `None` lists every source.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> impl Future<Output = Result<Vec<SourceListing>, CoreError>> + Send;

    /// Across organizations: the token alone designates the source.
    fn find_by_token(
        &mut self,
        hash: &TokenHash,
    ) -> impl Future<Output = Result<Option<Source>, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait SourceEventRepository: Send {
    fn insert(&mut self, event: &SourceEvent)
    -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Saves the outcome: status, card, run and error.
    fn save_outcome(
        &mut self,
        event: &SourceEvent,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Newest first, without their payload, with the total count.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        source_id: SourceId,
        limit: i64,
        offset: i64,
    ) -> impl Future<Output = Result<(Vec<SourceEvent>, i64), CoreError>> + Send;

    fn count_since(
        &mut self,
        source_id: SourceId,
        since: DateTime<Utc>,
    ) -> impl Future<Output = Result<i64, CoreError>> + Send;
}
