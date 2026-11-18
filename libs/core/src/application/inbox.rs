use capitainerie_common::{CoreError, InboxItemId};
use capitainerie_macros::transactional;
use chrono::Utc;

use super::CapitainerieUseCase;
use crate::domain::{
    authorization::Subject,
    inbox::{InboxPage, ReadSelection, projection::InboxProjection, service::InboxService},
};

impl CapitainerieUseCase {
    #[transactional(inbox, authz)]
    pub async fn list_inbox(
        &self,
        subject: Subject,
        unread_only: bool,
        before: Option<InboxItemId>,
        limit: u32,
    ) -> Result<InboxPage, CoreError> {
        InboxService::new(inbox_repository, authz)
            .list(subject, unread_only, before, limit)
            .await
    }

    /// Returns `(updated, unread)`.
    #[transactional(inbox, authz)]
    pub async fn mark_inbox_read(
        &self,
        subject: Subject,
        selection: ReadSelection,
    ) -> Result<(u64, i64), CoreError> {
        InboxService::new(inbox_repository, authz)
            .mark_read(subject, selection)
            .await
    }

    /// Projects one batch of outbox events into the inbox; returns how many
    /// events it read. Several workers may call it: the cursor is locked.
    #[transactional(inbox_feed, emitter)]
    pub async fn project_inbox(&self, batch: i64, settle_secs: i64) -> Result<usize, CoreError> {
        InboxProjection::new(inbox_feed_repository, emitter)
            .project(batch, settle_secs, Utc::now())
            .await
    }
}
