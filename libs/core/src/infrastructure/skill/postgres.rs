use capitainerie_common::{CoreError, OrganizationId, SkillId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        crew::Slug,
        skill::{CatalogSkill, Skill, SkillSource, ports::SkillRepository},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const LIST_LIMIT: i64 = 500;

#[repository(domain = Skill, backend = Postgres)]
pub struct PgSkillRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgSkillRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl SkillRepository for PgSkillRepository<'_> {
    async fn insert(&mut self, skill: &Skill) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO skills (
                id, organization_id, slug, name, description, content, source, archived,
                created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            skill.id.0,
            skill.organization_id.0,
            skill.slug.as_str(),
            skill.name,
            skill.description,
            skill.content,
            skill.source.as_str(),
            skill.archived,
            skill.created_by,
            skill.created_at,
            skill.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| named(error, skill))
    }

    async fn update(&mut self, skill: &Skill) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE skills
            SET slug = $3, name = $4, description = $5, content = $6, source = $7,
                archived = $8, updated_at = $9
            WHERE organization_id = $1 AND id = $2
            "#,
            skill.organization_id.0,
            skill.id.0,
            skill.slug.as_str(),
            skill.name,
            skill.description,
            skill.content,
            skill.source.as_str(),
            skill.archived,
            skill.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(|error| named(error, skill))?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: SkillId,
    ) -> Result<Option<CatalogSkill>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            CatalogRow,
            r#"
            SELECT s.id, s.organization_id, s.slug, s.name, s.description, s.content, s.source,
                   s.archived, s.created_by, s.created_at, s.updated_at,
                   (SELECT count(*) FROM agents a
                    WHERE a.organization_id = s.organization_id AND NOT a.archived
                      AND a.skills @> ARRAY[s.slug]) AS "used_by!"
            FROM skills s
            WHERE s.organization_id = $1 AND s.id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(CatalogSkill::try_from)
        .transpose()
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> Result<Vec<CatalogSkill>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            CatalogRow,
            r#"
            SELECT s.id, s.organization_id, s.slug, s.name, s.description, s.content, s.source,
                   s.archived, s.created_by, s.created_at, s.updated_at,
                   (SELECT count(*) FROM agents a
                    WHERE a.organization_id = s.organization_id AND NOT a.archived
                      AND a.skills @> ARRAY[s.slug]) AS "used_by!"
            FROM skills s
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
        .map(CatalogSkill::try_from)
        .collect()
    }

    async fn active_by_slugs(
        &mut self,
        organization_id: OrganizationId,
        slugs: &[String],
    ) -> Result<Vec<Skill>, CoreError> {
        if slugs.is_empty() {
            return Ok(Vec::new());
        }
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            SkillRow,
            r#"
            SELECT id, organization_id, slug, name, description, content, source, archived,
                   created_by, created_at, updated_at
            FROM skills
            WHERE organization_id = $1 AND NOT archived AND slug = ANY($2)
            "#,
            organization_id.0,
            slugs,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Skill::try_from)
        .collect()
    }
}

fn named(error: sqlx::Error, skill: &Skill) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.is_unique_violation() => CoreError::Conflict(format!(
            "a skill with the slug {} already exists",
            skill.slug.as_str()
        )),
        other => map_sqlx_error(other),
    }
}

struct SkillRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    description: String,
    content: String,
    source: String,
    archived: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<SkillRow> for Skill {
    type Error = CoreError;

    fn try_from(row: SkillRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored skill {}: {error}", row.id));
        Ok(Self {
            id: SkillId(row.id),
            organization_id: OrganizationId(row.organization_id),
            slug: Slug::try_from(row.slug).map_err(stored)?,
            name: row.name,
            description: row.description,
            content: row.content,
            source: SkillSource::try_from(row.source).map_err(stored)?,
            archived: row.archived,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

struct CatalogRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    description: String,
    content: String,
    source: String,
    archived: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    used_by: i64,
}

impl TryFrom<CatalogRow> for CatalogSkill {
    type Error = CoreError;

    fn try_from(row: CatalogRow) -> Result<Self, Self::Error> {
        let used_by = row.used_by;
        let skill = Skill::try_from(SkillRow {
            id: row.id,
            organization_id: row.organization_id,
            slug: row.slug,
            name: row.name,
            description: row.description,
            content: row.content,
            source: row.source,
            archived: row.archived,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })?;
        Ok(Self { skill, used_by })
    }
}
