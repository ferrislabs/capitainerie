use capitainerie_common::{AgentId, ArtifactId, CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::artifact::{
        Artifact, ArtifactFilter, ArtifactKind, ArtifactView, ports::ArtifactRepository,
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Artifact, backend = Postgres)]
pub struct PgArtifactRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgArtifactRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl ArtifactRepository for PgArtifactRepository<'_> {
    async fn insert(&mut self, artifact: &Artifact) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        let inserted = sqlx::query!(
            r#"
            INSERT INTO artifacts (id, organization_id, card_id, run_id, agent_id, kind, title,
                                   content, url, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (run_id) WHERE run_id IS NOT NULL DO NOTHING
            "#,
            artifact.id.0,
            artifact.organization_id.0,
            artifact.card_id.map(|id| id.0),
            artifact.run_id.map(|id| id.0),
            artifact.agent_id.map(|id| id.0),
            artifact.kind.as_str(),
            artifact.title,
            artifact.content,
            artifact.url,
            artifact.created_by,
            artifact.created_at,
            artifact.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(inserted.rows_affected() == 1)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ArtifactId,
    ) -> Result<Option<ArtifactView>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ArtifactRow,
            r#"
            SELECT x.id, x.organization_id, x.card_id, x.run_id, x.agent_id, x.kind, x.title,
                   x.content, x.url, x.created_by, x.created_at, x.updated_at,
                   a.name AS "agent_name?", c.title AS "card_title?"
            FROM artifacts x
            LEFT JOIN agents a ON a.id = x.agent_id
            LEFT JOIN cards c ON c.id = x.card_id
            WHERE x.organization_id = $1 AND x.id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(ArtifactView::try_from)
        .transpose()
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &ArtifactFilter,
    ) -> Result<Vec<ArtifactView>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ArtifactRow,
            r#"
            SELECT x.id, x.organization_id, x.card_id, x.run_id, x.agent_id, x.kind, x.title,
                   NULL::text AS content, x.url, x.created_by, x.created_at, x.updated_at,
                   a.name AS "agent_name?", c.title AS "card_title?"
            FROM artifacts x
            LEFT JOIN agents a ON a.id = x.agent_id
            LEFT JOIN cards c ON c.id = x.card_id
            WHERE x.organization_id = $1
              AND ($2::uuid IS NULL OR x.card_id = $2)
              AND ($3::uuid IS NULL OR x.agent_id = $3)
              AND ($4::text IS NULL OR x.kind = $4)
              AND ($5::uuid IS NULL OR x.id < $5)
            ORDER BY x.id DESC
            LIMIT $6
            "#,
            organization_id.0,
            filter.card_id.map(|id| id.0),
            filter.agent_id.map(|id| id.0),
            filter.kind.map(ArtifactKind::as_str),
            filter.before.map(|id| id.0),
            i64::from(filter.limit),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(ArtifactView::try_from)
        .collect()
    }
}

struct ArtifactRow {
    id: Uuid,
    organization_id: Uuid,
    card_id: Option<Uuid>,
    run_id: Option<Uuid>,
    agent_id: Option<Uuid>,
    kind: String,
    title: String,
    content: Option<String>,
    url: Option<String>,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    agent_name: Option<String>,
    card_title: Option<String>,
}

impl TryFrom<ArtifactRow> for ArtifactView {
    type Error = CoreError;

    fn try_from(row: ArtifactRow) -> Result<Self, Self::Error> {
        Ok(Self {
            artifact: Artifact {
                id: ArtifactId(row.id),
                organization_id: OrganizationId(row.organization_id),
                card_id: row.card_id.map(CardId),
                run_id: row.run_id.map(RunId),
                agent_id: row.agent_id.map(AgentId),
                kind: ArtifactKind::try_from(row.kind.as_str()).map_err(|error| {
                    CoreError::Internal(format!("stored artifact {}: {error}", row.id))
                })?,
                title: row.title,
                content: row.content,
                url: row.url,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            agent_name: row.agent_name,
            card_title: row.card_title,
        })
    }
}
