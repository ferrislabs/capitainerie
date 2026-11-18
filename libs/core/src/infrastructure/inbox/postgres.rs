use capitainerie_common::{CardId, CoreError, InboxItemId, OrganizationId, RunId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::inbox::{InboxItem, InboxKind, ReadSelection, ports::InboxRepository},
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Inbox, backend = Postgres)]
pub struct PgInboxRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgInboxRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl InboxRepository for PgInboxRepository<'_> {
    async fn list(
        &mut self,
        organization_id: OrganizationId,
        member: &str,
        unread_only: bool,
        before: Option<InboxItemId>,
        limit: u32,
    ) -> Result<Vec<InboxItem>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            InboxRow,
            r#"
            SELECT id, organization_id, member_subject, event_id, kind, title, body, link,
                   card_id, run_id, created_at, read_at
            FROM inbox_items
            WHERE organization_id = $1 AND member_subject = $2
              AND (NOT $3 OR read_at IS NULL)
              AND ($4::uuid IS NULL OR id < $4)
            ORDER BY id DESC
            LIMIT $5
            "#,
            organization_id.0,
            member,
            unread_only,
            before.map(|id| id.0),
            i64::from(limit),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(InboxItem::try_from)
        .collect()
    }

    async fn counts(
        &mut self,
        organization_id: OrganizationId,
        member: &str,
    ) -> Result<(i64, i64), CoreError> {
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT count(*) AS "total!", count(*) FILTER (WHERE read_at IS NULL) AS "unread!"
            FROM inbox_items
            WHERE organization_id = $1 AND member_subject = $2
            "#,
            organization_id.0,
            member,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok((row.total, row.unread))
    }

    async fn mark_read(
        &mut self,
        organization_id: OrganizationId,
        member: &str,
        selection: &ReadSelection,
        now: DateTime<Utc>,
    ) -> Result<u64, CoreError> {
        let (all, ids): (bool, Vec<Uuid>) = match selection {
            ReadSelection::All => (true, Vec::new()),
            ReadSelection::Some(ids) => (false, ids.iter().map(|id| id.0).collect()),
        };
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE inbox_items SET read_at = $5
            WHERE organization_id = $1 AND member_subject = $2 AND read_at IS NULL
              AND ($3 OR id = ANY($4))
            "#,
            organization_id.0,
            member,
            all,
            &ids,
            now,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(updated.rows_affected())
    }
}

pub(super) struct InboxRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub member_subject: String,
    pub event_id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub card_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

impl TryFrom<InboxRow> for InboxItem {
    type Error = CoreError;

    fn try_from(row: InboxRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: InboxItemId(row.id),
            organization_id: OrganizationId(row.organization_id),
            member_subject: row.member_subject,
            event_id: row.event_id,
            kind: InboxKind::try_from(row.kind.as_str()).map_err(|error| {
                CoreError::Internal(format!("stored inbox item {}: {error}", row.id))
            })?,
            title: row.title,
            body: row.body,
            link: row.link,
            card_id: row.card_id.map(CardId),
            run_id: row.run_id.map(RunId),
            created_at: row.created_at,
            read_at: row.read_at,
        })
    }
}
