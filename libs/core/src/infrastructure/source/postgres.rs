use capitainerie_common::{
    AgentId, CardId, CoreError, OrganizationId, ProjectId, RunId, SourceEventId, SourceId,
};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    domain::{
        crew::Slug,
        source::{
            Source, SourceConfig, SourceEvent, SourceEventStatus, SourceKind, SourceListing,
            TokenHash,
            ports::{SourceEventRepository, SourceRepository},
        },
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const LIST_LIMIT: i64 = 500;

#[repository(domain = Source, backend = Postgres)]
pub struct PgSourceRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgSourceRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl SourceRepository for PgSourceRepository<'_> {
    async fn insert(&mut self, source: &Source) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO sources (
                id, organization_id, slug, name, kind, description, enabled, config,
                webhook_token_hash, default_agent_id, project_id, auto_run, allow_writes,
                archived, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            "#,
            source.id.0,
            source.organization_id.0,
            source.slug.as_str(),
            source.name,
            source.kind.as_str(),
            source.description,
            source.enabled,
            source.config.as_value(),
            source.webhook_token_hash.as_str(),
            source.default_agent_id.map(|id| id.0),
            source.project_id.map(|id| id.0),
            source.auto_run,
            source.allow_writes,
            source.archived,
            source.created_by,
            source.created_at,
            source.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| named(error, source))
    }

    async fn update(&mut self, source: &Source) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE sources
            SET slug = $3, name = $4, description = $5, enabled = $6, config = $7,
                webhook_token_hash = $8, default_agent_id = $9, project_id = $10,
                auto_run = $11, allow_writes = $12, archived = $13, updated_at = $14
            WHERE organization_id = $1 AND id = $2
            "#,
            source.organization_id.0,
            source.id.0,
            source.slug.as_str(),
            source.name,
            source.description,
            source.enabled,
            source.config.as_value(),
            source.webhook_token_hash.as_str(),
            source.default_agent_id.map(|id| id.0),
            source.project_id.map(|id| id.0),
            source.auto_run,
            source.allow_writes,
            source.archived,
            source.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(|error| named(error, source))?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: SourceId,
    ) -> Result<Option<SourceListing>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ListingRow,
            r#"
            SELECT s.id, s.organization_id, s.slug, s.name, s.kind, s.description, s.enabled,
                   s.config, s.webhook_token_hash, s.default_agent_id, s.project_id, s.auto_run,
                   s.allow_writes, s.archived, s.created_by, s.created_at, s.updated_at,
                   (SELECT count(*) FROM source_events e
                    WHERE e.source_id = s.id AND e.received_at > now() - interval '24 hours')
                       AS "events_last_24h!",
                   (SELECT max(e.received_at) FROM source_events e WHERE e.source_id = s.id)
                       AS last_event_at
            FROM sources s
            WHERE s.organization_id = $1 AND s.id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(SourceListing::try_from)
        .transpose()
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> Result<Vec<SourceListing>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ListingRow,
            r#"
            SELECT s.id, s.organization_id, s.slug, s.name, s.kind, s.description, s.enabled,
                   s.config, s.webhook_token_hash, s.default_agent_id, s.project_id, s.auto_run,
                   s.allow_writes, s.archived, s.created_by, s.created_at, s.updated_at,
                   (SELECT count(*) FROM source_events e
                    WHERE e.source_id = s.id AND e.received_at > now() - interval '24 hours')
                       AS "events_last_24h!",
                   (SELECT max(e.received_at) FROM source_events e WHERE e.source_id = s.id)
                       AS last_event_at
            FROM sources s
            WHERE s.organization_id = $1 AND ($2::bool IS NULL OR s.archived = $2)
            ORDER BY lower(s.name), s.id
            LIMIT $3
            "#,
            organization_id.0,
            archived,
            LIST_LIMIT,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(SourceListing::try_from)
        .collect()
    }

    async fn find_by_token(&mut self, hash: &TokenHash) -> Result<Option<Source>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            SourceRow,
            r#"
            SELECT id, organization_id, slug, name, kind, description, enabled, config,
                   webhook_token_hash, default_agent_id, project_id, auto_run, allow_writes,
                   archived, created_by, created_at, updated_at
            FROM sources
            WHERE webhook_token_hash = $1
            "#,
            hash.as_str(),
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Source::try_from)
        .transpose()
    }
}

fn named(error: sqlx::Error, source: &Source) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.constraint() == Some("sources_slug_unique") => {
            CoreError::Conflict(format!(
                "a source with the slug {} already exists",
                source.slug.as_str()
            ))
        }
        other => map_sqlx_error(other),
    }
}

struct SourceRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    kind: String,
    description: String,
    enabled: bool,
    config: Value,
    webhook_token_hash: String,
    default_agent_id: Option<Uuid>,
    project_id: Option<Uuid>,
    auto_run: bool,
    allow_writes: bool,
    archived: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<SourceRow> for Source {
    type Error = CoreError;

    fn try_from(row: SourceRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored source {}: {error}", row.id));
        Ok(Self {
            id: SourceId(row.id),
            organization_id: OrganizationId(row.organization_id),
            slug: Slug::try_from(row.slug).map_err(stored)?,
            name: row.name,
            kind: SourceKind::try_from(row.kind.as_str()).map_err(stored)?,
            description: row.description,
            enabled: row.enabled,
            config: SourceConfig::try_from(row.config).map_err(stored)?,
            webhook_token_hash: TokenHash::from_stored(row.webhook_token_hash).map_err(stored)?,
            default_agent_id: row.default_agent_id.map(AgentId),
            project_id: row.project_id.map(ProjectId),
            auto_run: row.auto_run,
            allow_writes: row.allow_writes,
            archived: row.archived,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

struct ListingRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    kind: String,
    description: String,
    enabled: bool,
    config: Value,
    webhook_token_hash: String,
    default_agent_id: Option<Uuid>,
    project_id: Option<Uuid>,
    auto_run: bool,
    allow_writes: bool,
    archived: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    events_last_24h: i64,
    last_event_at: Option<DateTime<Utc>>,
}

impl TryFrom<ListingRow> for SourceListing {
    type Error = CoreError;

    fn try_from(row: ListingRow) -> Result<Self, Self::Error> {
        let events_last_24h = row.events_last_24h;
        let last_event_at = row.last_event_at;
        let source = Source::try_from(SourceRow {
            id: row.id,
            organization_id: row.organization_id,
            slug: row.slug,
            name: row.name,
            kind: row.kind,
            description: row.description,
            enabled: row.enabled,
            config: row.config,
            webhook_token_hash: row.webhook_token_hash,
            default_agent_id: row.default_agent_id,
            project_id: row.project_id,
            auto_run: row.auto_run,
            allow_writes: row.allow_writes,
            archived: row.archived,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })?;
        Ok(Self {
            source,
            events_last_24h,
            last_event_at,
        })
    }
}

#[repository(domain = SourceEvent, backend = Postgres)]
pub struct PgSourceEventRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgSourceEventRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl SourceEventRepository for PgSourceEventRepository<'_> {
    async fn insert(&mut self, event: &SourceEvent) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO source_events (
                id, organization_id, source_id, received_at, status, payload, summary,
                card_id, run_id, error
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            event.id.0,
            event.organization_id.0,
            event.source_id.0,
            event.received_at,
            event.status.as_str(),
            event.payload,
            event.summary,
            event.card_id.map(|id| id.0),
            event.run_id.map(|id| id.0),
            event.error,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn save_outcome(&mut self, event: &SourceEvent) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE source_events
            SET status = $3, card_id = $4, run_id = $5, error = $6
            WHERE organization_id = $1 AND id = $2
            "#,
            event.organization_id.0,
            event.id.0,
            event.status.as_str(),
            event.card_id.map(|id| id.0),
            event.run_id.map(|id| id.0),
            event.error,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        source_id: SourceId,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<SourceEvent>, i64), CoreError> {
        let mut tx = self.tx.lock().await;
        let total = sqlx::query_scalar!(
            r#"
            SELECT count(*) AS "count!" FROM source_events
            WHERE organization_id = $1 AND source_id = $2
            "#,
            organization_id.0,
            source_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        let events = sqlx::query_as!(
            EventRow,
            r#"
            SELECT id, organization_id, source_id, received_at, status,
                   'null'::jsonb AS "payload!", summary, card_id, run_id, error
            FROM source_events
            WHERE organization_id = $1 AND source_id = $2
            ORDER BY received_at DESC, id DESC
            LIMIT $3 OFFSET $4
            "#,
            organization_id.0,
            source_id.0,
            limit,
            offset,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(SourceEvent::try_from)
        .collect::<Result<Vec<_>, _>>()?;
        Ok((events, total))
    }

    async fn count_since(
        &mut self,
        source_id: SourceId,
        since: DateTime<Utc>,
    ) -> Result<i64, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"
            SELECT count(*) AS "count!" FROM source_events
            WHERE source_id = $1 AND received_at > $2
            "#,
            source_id.0,
            since,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }
}

struct EventRow {
    id: Uuid,
    organization_id: Uuid,
    source_id: Uuid,
    received_at: DateTime<Utc>,
    status: String,
    payload: Value,
    summary: String,
    card_id: Option<Uuid>,
    run_id: Option<Uuid>,
    error: Option<String>,
}

impl TryFrom<EventRow> for SourceEvent {
    type Error = CoreError;

    fn try_from(row: EventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: SourceEventId(row.id),
            organization_id: OrganizationId(row.organization_id),
            source_id: SourceId(row.source_id),
            received_at: row.received_at,
            status: SourceEventStatus::try_from(row.status.as_str()).map_err(|error| {
                CoreError::Internal(format!("stored source event {}: {error}", row.id))
            })?,
            payload: row.payload,
            summary: row.summary,
            card_id: row.card_id.map(CardId),
            run_id: row.run_id.map(RunId),
            error: row.error,
        })
    }
}
