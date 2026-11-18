use std::collections::HashMap;

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_events::EventEnvelope;
use capitainerie_macros::repository;
use uuid::Uuid;

use crate::{
    domain::audit::{
        AuditEvent, AuditFilter,
        ports::{AuditRepository, NamedAgent},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Audit, backend = Postgres)]
pub struct PgAuditRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgAuditRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl AuditRepository for PgAuditRepository<'_> {
    async fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &AuditFilter,
    ) -> Result<Vec<AuditEvent>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT o.id, o.topic, o.envelope
            FROM outbox_events o
            WHERE o.organization_id = $1
              AND o.topic <> 'inbox'
              AND ($2::text IS NULL OR o.topic = $2)
              AND ($3::text IS NULL OR o.kind = $3)
              AND ($4::text IS NULL
                   OR o.envelope->'payload'->>'initiated_by' = $4
                   OR o.envelope->'payload'->'author'->>'subject' = $4
                   OR o.envelope->'payload'->>'decided_by' = $4)
              AND ($5::uuid IS NULL
                   OR o.envelope->'payload'->>'requested_by_agent_id' = $5::text
                   OR o.envelope->'payload'->'author'->>'actor' =
                      (SELECT 'agent:' || a.slug FROM agents a WHERE a.id = $5 AND a.organization_id = $1)
                   OR o.envelope->'payload'->>'run_id' IN
                      (SELECT t.run_id::text FROM agent_tasks t WHERE t.agent_id = $5 AND t.organization_id = $1))
              AND ($6::uuid IS NULL OR o.envelope->'payload'->>'card_id' = $6::text)
              AND ($7::timestamptz IS NULL OR o.created_at >= $7)
              AND ($8::timestamptz IS NULL OR o.created_at < $8)
              AND ($9::uuid IS NULL OR o.id < $9)
            ORDER BY o.id DESC
            LIMIT $10
            "#,
            organization_id.0,
            filter.topic,
            filter.kind,
            filter.actor,
            filter.agent_id.map(|id| id.0),
            filter.card_id.map(|id| id.0),
            filter.from,
            filter.to,
            filter.before,
            i64::from(filter.limit),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| {
                let envelope: EventEnvelope =
                    serde_json::from_value(row.envelope).map_err(|error| {
                        CoreError::Internal(format!("decoding outbox event {}: {error}", row.id))
                    })?;
                Ok(AuditEvent {
                    id: row.id,
                    topic: row.topic,
                    kind: envelope.kind,
                    occurred_at: envelope.occurred_at,
                    payload: envelope.payload,
                })
            })
            .collect()
    }

    async fn member_names(
        &mut self,
        organization_id: OrganizationId,
        subjects: &[String],
    ) -> Result<HashMap<String, String>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            "SELECT subject, name FROM members WHERE organization_id = $1 AND subject = ANY($2)",
            organization_id.0,
            subjects,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| (row.subject, row.name))
            .collect())
    }

    async fn agents_by_id(
        &mut self,
        organization_id: OrganizationId,
        ids: &[Uuid],
    ) -> Result<Vec<NamedAgent>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            NamedAgent,
            "SELECT id, slug, name FROM agents WHERE organization_id = $1 AND id = ANY($2)",
            organization_id.0,
            ids,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn agents_by_slug(
        &mut self,
        organization_id: OrganizationId,
        slugs: &[String],
    ) -> Result<Vec<NamedAgent>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            NamedAgent,
            "SELECT id, slug, name FROM agents WHERE organization_id = $1 AND slug = ANY($2)",
            organization_id.0,
            slugs,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn run_agents(
        &mut self,
        organization_id: OrganizationId,
        run_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, NamedAgent>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT t.run_id, a.id, a.slug, a.name
            FROM agent_tasks t
            JOIN agents a ON a.id = t.agent_id
            WHERE t.organization_id = $1 AND t.run_id = ANY($2)
            "#,
            organization_id.0,
            run_ids,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.run_id,
                    NamedAgent {
                        id: row.id,
                        slug: row.slug,
                        name: row.name,
                    },
                )
            })
            .collect())
    }

    async fn card_titles(
        &mut self,
        organization_id: OrganizationId,
        card_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            "SELECT id, title FROM cards WHERE organization_id = $1 AND id = ANY($2)",
            organization_id.0,
            card_ids,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(|row| (row.id, row.title)).collect())
    }
}
