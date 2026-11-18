use capitainerie_common::{AgentId, CoreError, OrganizationId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    domain::crew::{
        AgentConfig, AgentRevision, AgentStatus, ConnectorGrants, CrewAgent, Hue, Skills, Slug,
        ports::CrewRepository,
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const LIST_LIMIT: i64 = 500;

#[repository(domain = CrewAgent, backend = Postgres)]
pub struct PgCrewRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgCrewRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl CrewRepository for PgCrewRepository<'_> {
    async fn insert(&mut self, agent: &CrewAgent) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO agents (
                id, organization_id, slug, name, role, description, model, status, hue,
                reports_to, archived, instructions, connector_grants, config, skills, paused, revision,
                   created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            "#,
            agent.id.0,
            agent.organization_id.0,
            agent.slug.as_str(),
            agent.name,
            agent.role,
            agent.description,
            agent.model,
            agent.status.as_str(),
            agent.hue.get(),
            agent.reports_to.map(|id| id.0),
            agent.archived,
            agent.instructions,
            agent.connector_grants.as_slice(),
            agent.config.to_value(),
            agent.skills.as_slice(),
            agent.paused,
            agent.revision,
            agent.created_by,
            agent.created_at,
            agent.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| named(error, agent))
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: AgentId,
    ) -> Result<Option<CrewAgent>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            AgentRow,
            r#"
            SELECT id, organization_id, slug, name, role, description, model, status, hue,
                   reports_to, archived, instructions, connector_grants, config, skills, paused, revision,
                   created_by, created_at, updated_at
            FROM agents
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(CrewAgent::try_from)
        .transpose()
    }

    async fn update(&mut self, agent: &CrewAgent) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE agents
            SET slug = $3, name = $4, role = $5, description = $6, model = $7, status = $8,
                hue = $9, reports_to = $10, archived = $11, instructions = $12,
                connector_grants = $13, config = $14, skills = $15, paused = $16,
                revision = $17, updated_at = $18
            WHERE organization_id = $1 AND id = $2
            "#,
            agent.organization_id.0,
            agent.id.0,
            agent.slug.as_str(),
            agent.name,
            agent.role,
            agent.description,
            agent.model,
            agent.status.as_str(),
            agent.hue.get(),
            agent.reports_to.map(|id| id.0),
            agent.archived,
            agent.instructions,
            agent.connector_grants.as_slice(),
            agent.config.to_value(),
            agent.skills.as_slice(),
            agent.paused,
            agent.revision,
            agent.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(|error| named(error, agent))?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> Result<Vec<CrewAgent>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            AgentRow,
            r#"
            SELECT id, organization_id, slug, name, role, description, model, status, hue,
                   reports_to, archived, instructions, connector_grants, config, skills, paused, revision,
                   created_by, created_at, updated_at
            FROM agents
            WHERE organization_id = $1 AND ($2::bool IS NULL OR archived = $2)
            ORDER BY lower(name), id
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
        .map(CrewAgent::try_from)
        .collect()
    }

    async fn insert_revision(&mut self, revision: &AgentRevision) -> Result<(), CoreError> {
        let snapshot = serde_json::to_value(&revision.snapshot)
            .map_err(|error| CoreError::Internal(format!("agent snapshot: {error}")))?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO agent_revisions (
                id, agent_id, organization_id, revision, snapshot, changed, author, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            revision.id,
            revision.agent_id.0,
            revision.organization_id.0,
            revision.revision,
            snapshot,
            revision.changed,
            revision.author,
            revision.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn list_revisions(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
    ) -> Result<Vec<AgentRevision>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RevisionRow,
            r#"
            SELECT id, agent_id, organization_id, revision, snapshot, changed, author, created_at
            FROM agent_revisions
            WHERE organization_id = $1 AND agent_id = $2
            ORDER BY revision DESC
            LIMIT $3
            "#,
            organization_id.0,
            agent_id.0,
            LIST_LIMIT,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(AgentRevision::try_from)
        .collect()
    }

    async fn find_revision(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
        revision: i32,
    ) -> Result<Option<AgentRevision>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RevisionRow,
            r#"
            SELECT id, agent_id, organization_id, revision, snapshot, changed, author, created_at
            FROM agent_revisions
            WHERE organization_id = $1 AND agent_id = $2 AND revision = $3
            "#,
            organization_id.0,
            agent_id.0,
            revision,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(AgentRevision::try_from)
        .transpose()
    }
}

struct RevisionRow {
    id: Uuid,
    agent_id: Uuid,
    organization_id: Uuid,
    revision: i32,
    snapshot: Value,
    changed: String,
    author: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<RevisionRow> for AgentRevision {
    type Error = CoreError;

    fn try_from(row: RevisionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            agent_id: AgentId(row.agent_id),
            organization_id: OrganizationId(row.organization_id),
            revision: row.revision,
            snapshot: serde_json::from_value(row.snapshot).map_err(|error| {
                CoreError::Internal(format!("stored agent revision {}: {error}", row.id))
            })?,
            changed: row.changed,
            author: row.author,
            created_at: row.created_at,
        })
    }
}

fn named(error: sqlx::Error, agent: &CrewAgent) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.is_unique_violation() => CoreError::Conflict(format!(
            "an agent with the slug {} already exists",
            agent.slug.as_str()
        )),
        other => map_sqlx_error(other),
    }
}

struct AgentRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    role: String,
    description: String,
    model: String,
    status: String,
    hue: i16,
    reports_to: Option<Uuid>,
    archived: bool,
    instructions: String,
    connector_grants: Vec<String>,
    config: Value,
    skills: Vec<String>,
    paused: bool,
    revision: i32,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<AgentRow> for CrewAgent {
    type Error = CoreError;

    fn try_from(row: AgentRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored agent {}: {error}", row.id));
        Ok(Self {
            id: AgentId(row.id),
            organization_id: OrganizationId(row.organization_id),
            slug: Slug::try_from(row.slug).map_err(stored)?,
            name: row.name,
            role: row.role,
            description: row.description,
            model: row.model,
            status: AgentStatus::try_from(row.status.as_str()).map_err(stored)?,
            hue: Hue::try_from(row.hue).map_err(stored)?,
            reports_to: row.reports_to.map(AgentId),
            archived: row.archived,
            instructions: row.instructions,
            connector_grants: ConnectorGrants::try_from(row.connector_grants).map_err(stored)?,
            config: AgentConfig::try_from(row.config).map_err(stored)?,
            skills: Skills::try_from(row.skills).map_err(stored)?,
            paused: row.paused,
            revision: row.revision,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
