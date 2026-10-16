use capitainerie_common::CoreError;
use capitainerie_events::EventEnvelope;
use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::postgres::{SharedTx, map_sqlx_error};

pub const OUTBOX_CHANNEL: &str = "outbox_events";

pub struct PgOutboxRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgOutboxRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }

    /// Appends the events and wakes the relay. `NOTIFY` is delivered on commit
    /// only, so the relay is never woken for a rolled-back transaction.
    pub async fn append(&mut self, events: &[EventEnvelope]) -> Result<(), CoreError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut tx = self.tx.lock().await;
        for event in events {
            let envelope = serde_json::to_value(event)
                .map_err(|error| CoreError::Internal(format!("encoding outbox event: {error}")))?;
            sqlx::query!(
                r#"
                INSERT INTO outbox_events (id, organization_id, topic, kind, partition_key, envelope)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                event.id,
                event.organization_id,
                event.topic.as_str(),
                event.kind,
                event.partition_key,
                envelope,
            )
            .execute(&mut ***tx)
            .await
            .map_err(map_sqlx_error)?;
        }
        sqlx::query!("SELECT pg_notify($1, '')", OUTBOX_CHANNEL)
            .execute(&mut ***tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(())
    }
}

/// Locks up to `limit` pending events, oldest first. Rows locked by another
/// relay are skipped, so several relays never publish the same batch at once.
pub(super) async fn claim_pending(
    tx: &mut sqlx::PgConnection,
    limit: i64,
) -> Result<Vec<(Uuid, EventEnvelope)>, CoreError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, envelope
        FROM outbox_events
        WHERE published_at IS NULL
        ORDER BY id
        LIMIT $1
        FOR UPDATE SKIP LOCKED
        "#,
        limit,
    )
    .fetch_all(tx)
    .await
    .map_err(map_sqlx_error)?;

    rows.into_iter()
        .map(|row| {
            serde_json::from_value(row.envelope)
                .map(|envelope| (row.id, envelope))
                .map_err(|error| {
                    CoreError::Internal(format!("decoding outbox event {}: {error}", row.id))
                })
        })
        .collect()
}

pub(super) async fn mark_published(
    tx: &mut sqlx::PgConnection,
    ids: &[Uuid],
) -> Result<(), CoreError> {
    sqlx::query!(
        "UPDATE outbox_events SET published_at = now() WHERE id = ANY($1)",
        ids,
    )
    .execute(tx)
    .await
    .map(|_| ())
    .map_err(map_sqlx_error)
}

pub async fn pending_count(pool: &PgPool) -> Result<i64, CoreError> {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM outbox_events WHERE published_at IS NULL"#
    )
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_error)
}
