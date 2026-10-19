use capitainerie_common::{ClientId, CoreError, OrganizationId, ProjectId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        card::Column,
        client::{ClientRef, ColumnCount},
        label::{Aliases, Name},
        project::{Project, ProjectFilter, ProjectListing, ports::ProjectRepository},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const LIST_LIMIT: i64 = 500;

#[repository(domain = Project, backend = Postgres)]
pub struct PgProjectRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgProjectRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl ProjectRepository for PgProjectRepository<'_> {
    async fn insert(&mut self, project: &Project) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO projects (
                id, organization_id, client_id, name, aliases, repository, description,
                created_by, created_at, updated_at, archived_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            project.id.0,
            project.organization_id.0,
            project.client_id().map(|id| id.0),
            project.name.as_str(),
            project.aliases.as_slice(),
            project.repository.as_ref().map(ToString::to_string),
            project.description,
            project.created_by,
            project.created_at,
            project.updated_at,
            project.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| named(error, project))
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ProjectId,
    ) -> Result<Option<Project>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ProjectRow,
            r#"
            SELECT p.id, p.organization_id, p.client_id, cl.name AS "client_name?", p.name, p.aliases,
                   p.repository, p.description, p.created_by, p.created_at, p.updated_at, p.archived_at
            FROM projects p
            LEFT JOIN clients cl ON cl.id = p.client_id
            WHERE p.organization_id = $1 AND p.id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Project::try_from)
        .transpose()
    }

    async fn update(&mut self, project: &Project) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let client_id = project.client_id().map(|id| id.0);
        let updated = sqlx::query!(
            r#"
            UPDATE projects
            SET client_id = $3, name = $4, aliases = $5, repository = $6, description = $7,
                updated_at = $8, archived_at = $9
            WHERE organization_id = $1 AND id = $2
            "#,
            project.organization_id.0,
            project.id.0,
            client_id,
            project.name.as_str(),
            project.aliases.as_slice(),
            project.repository.as_ref().map(ToString::to_string),
            project.description,
            project.updated_at,
            project.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(|error| named(error, project))?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        sqlx::query!(
            r#"
            UPDATE cards SET client_id = $3
            WHERE organization_id = $1 AND project_id = $2 AND client_id IS DISTINCT FROM $3
            "#,
            project.organization_id.0,
            project.id.0,
            client_id,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query!(
            r#"
            UPDATE transcripts SET client_id = $3
            WHERE organization_id = $1 AND project_id = $2 AND client_id IS DISTINCT FROM $3
            "#,
            project.organization_id.0,
            project.id.0,
            client_id,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &ProjectFilter,
    ) -> Result<Vec<ProjectListing>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ListingRow,
            r#"
            SELECT p.id, p.organization_id, p.client_id, cl.name AS "client_name?", p.name, p.aliases,
                   p.repository, p.description, p.created_by, p.created_at, p.updated_at, p.archived_at,
                   (SELECT count(*) FROM cards k
                     WHERE k.organization_id = p.organization_id AND k.project_id = p.id
                       AND k."column" IN ('proposed', 'todo', 'in_progress')) AS "open_cards!"
            FROM projects p
            LEFT JOIN clients cl ON cl.id = p.client_id
            WHERE p.organization_id = $1
              AND (p.archived_at IS NOT NULL) = $2
              AND ($3::uuid IS NULL OR p.client_id = $3)
              AND ($4::text IS NULL
                   OR strpos(lower(p.name), lower($4)) > 0
                   OR EXISTS (SELECT 1 FROM unnest(p.aliases) AS alias WHERE strpos(lower(alias), lower($4)) > 0))
            ORDER BY lower(p.name), p.id
            LIMIT $5
            "#,
            organization_id.0,
            filter.only_archived(),
            filter.client_id.map(|id| id.0),
            filter.needle(),
            LIST_LIMIT,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(ProjectListing::try_from)
        .collect()
    }

    async fn of_client(
        &mut self,
        organization_id: OrganizationId,
        client_id: ClientId,
    ) -> Result<Vec<Project>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ProjectRow,
            r#"
            SELECT p.id, p.organization_id, p.client_id, cl.name AS "client_name?", p.name, p.aliases,
                   p.repository, p.description, p.created_by, p.created_at, p.updated_at, p.archived_at
            FROM projects p
            LEFT JOIN clients cl ON cl.id = p.client_id
            WHERE p.organization_id = $1 AND p.client_id = $2 AND p.archived_at IS NULL
            ORDER BY lower(p.name), p.id
            "#,
            organization_id.0,
            client_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Project::try_from)
        .collect()
    }

    async fn card_counts(
        &mut self,
        organization_id: OrganizationId,
        id: ProjectId,
    ) -> Result<Vec<ColumnCount>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            SELECT "column" AS "column!", count(*) AS "cards!"
            FROM cards
            WHERE organization_id = $1 AND project_id = $2
            GROUP BY "column"
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(|row| {
            Ok(ColumnCount {
                column: Column::try_from(row.column.as_str())?,
                cards: u64::try_from(row.cards).unwrap_or_default(),
            })
        })
        .collect()
    }

    async fn recent(
        &mut self,
        organization_id: OrganizationId,
        limit: i64,
    ) -> Result<Vec<(DateTime<Utc>, Project)>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RecentRow,
            r#"
            SELECT p.id, p.organization_id, p.client_id, cl.name AS "client_name?", p.name, p.aliases,
                   p.repository, p.description, p.created_by, p.created_at, p.updated_at, p.archived_at,
                   GREATEST(
                       p.updated_at,
                       COALESCE((SELECT max(k.updated_at) FROM cards k
                                  WHERE k.organization_id = p.organization_id AND k.project_id = p.id), p.updated_at),
                       COALESCE((SELECT max(t.created_at) FROM transcripts t
                                  WHERE t.organization_id = p.organization_id AND t.project_id = p.id), p.updated_at)
                   ) AS "used_at!"
            FROM projects p
            LEFT JOIN clients cl ON cl.id = p.client_id
            WHERE p.organization_id = $1 AND p.archived_at IS NULL AND cl.archived_at IS NULL
            ORDER BY "used_at!" DESC, p.id
            LIMIT $2
            "#,
            organization_id.0,
            limit,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(<(DateTime<Utc>, Project)>::try_from)
        .collect()
    }
}

fn named(error: sqlx::Error, project: &Project) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.is_unique_violation() => CoreError::Conflict(format!(
            "a project named {} already exists for this client",
            project.name.as_str()
        )),
        other => map_sqlx_error(other),
    }
}

struct ProjectRow {
    id: Uuid,
    organization_id: Uuid,
    client_id: Option<Uuid>,
    client_name: Option<String>,
    name: String,
    aliases: Vec<String>,
    repository: Option<String>,
    description: String,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<ProjectRow> for Project {
    type Error = CoreError;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored project {}: {error}", row.id));
        Ok(Self {
            id: ProjectId(row.id),
            organization_id: OrganizationId(row.organization_id),
            client: row
                .client_id
                .zip(row.client_name)
                .map(|(id, name)| ClientRef {
                    id: ClientId(id),
                    name,
                }),
            name: Name::try_from(row.name).map_err(stored)?,
            aliases: Aliases::try_from(row.aliases).map_err(stored)?,
            repository: row
                .repository
                .as_deref()
                .map(str::parse)
                .transpose()
                .map_err(stored)?,
            description: row.description,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}

struct ListingRow {
    id: Uuid,
    organization_id: Uuid,
    client_id: Option<Uuid>,
    client_name: Option<String>,
    name: String,
    aliases: Vec<String>,
    repository: Option<String>,
    description: String,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    open_cards: i64,
}

impl TryFrom<ListingRow> for ProjectListing {
    type Error = CoreError;

    fn try_from(row: ListingRow) -> Result<Self, Self::Error> {
        let open_cards = u64::try_from(row.open_cards).unwrap_or_default();
        Ok(Self {
            project: Project::try_from(ProjectRow {
                id: row.id,
                organization_id: row.organization_id,
                client_id: row.client_id,
                client_name: row.client_name,
                name: row.name,
                aliases: row.aliases,
                repository: row.repository,
                description: row.description,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
                archived_at: row.archived_at,
            })?,
            open_cards,
        })
    }
}

struct RecentRow {
    id: Uuid,
    organization_id: Uuid,
    client_id: Option<Uuid>,
    client_name: Option<String>,
    name: String,
    aliases: Vec<String>,
    repository: Option<String>,
    description: String,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    used_at: DateTime<Utc>,
}

impl TryFrom<RecentRow> for (DateTime<Utc>, Project) {
    type Error = CoreError;

    fn try_from(row: RecentRow) -> Result<Self, Self::Error> {
        let used_at = row.used_at;
        Project::try_from(ProjectRow {
            id: row.id,
            organization_id: row.organization_id,
            client_id: row.client_id,
            client_name: row.client_name,
            name: row.name,
            aliases: row.aliases,
            repository: row.repository,
            description: row.description,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
        .map(|project| (used_at, project))
    }
}
