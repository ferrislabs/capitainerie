use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_events::EventEnvelope;
use capitainerie_macros::repository;
use uuid::Uuid;

use crate::{
    domain::inbox::{
        InboxItem,
        ports::{CardFacts, InboxFeed, OwnedFacts, RunFacts},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = InboxFeed, backend = Postgres)]
pub struct PgInboxFeed<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgInboxFeed<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl InboxFeed for PgInboxFeed<'_> {
    async fn lock_cursor(&mut self, name: &str) -> Result<Uuid, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO projection_cursors (name, last_event_id, updated_at)
            VALUES ($1, '00000000-0000-0000-0000-000000000000', now())
            ON CONFLICT (name) DO NOTHING
            "#,
            name,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query_scalar!(
            "SELECT last_event_id FROM projection_cursors WHERE name = $1 FOR UPDATE",
            name,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn events_after(
        &mut self,
        cursor: Uuid,
        settle_secs: i64,
        limit: i64,
    ) -> Result<Vec<EventEnvelope>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT id, envelope
            FROM outbox_events
            WHERE id > $1 AND created_at <= now() - $2::bigint * interval '1 second'
            ORDER BY id
            LIMIT $3
            "#,
            cursor,
            settle_secs,
            limit,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| {
                serde_json::from_value::<EventEnvelope>(row.envelope)
                    .map(|mut envelope| {
                        envelope.id = row.id;
                        envelope
                    })
                    .map_err(|error| {
                        CoreError::Internal(format!("decoding outbox event {}: {error}", row.id))
                    })
            })
            .collect()
    }

    async fn advance(&mut self, name: &str, to: Uuid) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "UPDATE projection_cursors SET last_event_id = $2, updated_at = now() WHERE name = $1",
            name,
            to,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn insert(&mut self, item: &InboxItem) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        let inserted = sqlx::query!(
            r#"
            INSERT INTO inbox_items (id, organization_id, member_subject, event_id, kind, title,
                                     body, link, card_id, run_id, created_at, read_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (organization_id, member_subject, event_id) DO NOTHING
            "#,
            item.id.0,
            item.organization_id.0,
            item.member_subject,
            item.event_id,
            item.kind.as_str(),
            item.title,
            item.body,
            item.link,
            item.card_id.map(|id| id.0),
            item.run_id.map(|id| id.0),
            item.created_at,
            item.read_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(inserted.rows_affected() == 1)
    }

    async fn run(
        &mut self,
        organization_id: OrganizationId,
        run_id: Uuid,
    ) -> Result<Option<RunFacts>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RunFacts,
            r#"
            SELECT r.initiated_by, a.name AS "agent_name?", r.card_id, c.title AS "card_title?"
            FROM runs r
            LEFT JOIN agent_tasks t ON t.run_id = r.id
            LEFT JOIN agents a ON a.id = t.agent_id
            LEFT JOIN cards c ON c.id = r.card_id
            WHERE r.organization_id = $1 AND r.id = $2
            "#,
            organization_id.0,
            run_id,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn card(
        &mut self,
        organization_id: OrganizationId,
        card_id: Uuid,
    ) -> Result<Option<CardFacts>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            CardFacts,
            r#"
            SELECT created_by_subject AS created_by, title
            FROM cards
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            card_id,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn source(
        &mut self,
        organization_id: OrganizationId,
        source_id: Uuid,
    ) -> Result<Option<OwnedFacts>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            OwnedFacts,
            "SELECT created_by, name FROM sources WHERE organization_id = $1 AND id = $2",
            organization_id.0,
            source_id,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn routine(
        &mut self,
        organization_id: OrganizationId,
        routine_id: Uuid,
    ) -> Result<Option<OwnedFacts>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            OwnedFacts,
            "SELECT created_by, name FROM routines WHERE organization_id = $1 AND id = $2",
            organization_id.0,
            routine_id,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn approval_title(
        &mut self,
        organization_id: OrganizationId,
        approval_id: Uuid,
    ) -> Result<Option<String>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            "SELECT title FROM approvals WHERE organization_id = $1 AND id = $2",
            organization_id.0,
            approval_id,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn agent_name(
        &mut self,
        organization_id: OrganizationId,
        slug: &str,
    ) -> Result<Option<String>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            "SELECT name FROM agents WHERE organization_id = $1 AND slug = $2",
            organization_id.0,
            slug,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }
}
