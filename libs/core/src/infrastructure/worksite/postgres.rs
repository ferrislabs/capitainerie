use capitainerie_common::{CardId, CoreError, OrganizationId, ProjectId, RunId};
use capitainerie_macros::repository;

use crate::{
    domain::{
        execution::RepositoryRef,
        worksite::{WorksiteIssue, WorksiteSummary, ports::WorksiteRepository},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Worksite, backend = Postgres)]
pub struct PgWorksiteRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgWorksiteRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl WorksiteRepository for PgWorksiteRepository<'_> {
    async fn insert_issue(&mut self, issue: &WorksiteIssue) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO worksite_issues (card_id, organization_id, repository, issue_number, issue_url, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            issue.card_id.0,
            issue.organization_id.0,
            issue.repository.to_string(),
            i64::try_from(issue.number).map_err(|_| CoreError::Invalid("issue number".into()))?,
            issue.url,
            issue.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| match map_sqlx_error(error) {
            CoreError::Conflict(_) => CoreError::Conflict("issue_already_imported".into()),
            other => other,
        })
    }

    async fn issues(
        &mut self,
        organization_id: OrganizationId,
        repository: &RepositoryRef,
        numbers: &[u64],
    ) -> Result<Vec<WorksiteIssue>, CoreError> {
        let numbers: Vec<i64> = numbers
            .iter()
            .filter_map(|number| i64::try_from(*number).ok())
            .collect();
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT card_id, issue_number, issue_url, created_at
            FROM worksite_issues
            WHERE organization_id = $1 AND repository = $2 AND issue_number = ANY($3)
            ORDER BY issue_number
            "#,
            organization_id.0,
            repository.to_string(),
            &numbers,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| WorksiteIssue {
                card_id: CardId(row.card_id),
                organization_id,
                repository: repository.clone(),
                number: row.issue_number as u64,
                url: row.issue_url,
                created_at: row.created_at,
            })
            .collect())
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        project_id: ProjectId,
    ) -> Result<Vec<WorksiteSummary>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT c.id, c.title, c."column", c.created_at,
                   w.repository AS "issue_repository?", w.issue_number AS "issue_number?", w.issue_url AS "issue_url?",
                   r.id AS "run_id?", r.status AS "run_status?", r.repository AS "run_repository?"
            FROM cards c
            LEFT JOIN worksite_issues w ON w.card_id = c.id
            LEFT JOIN LATERAL (
                SELECT id, status, repository
                FROM runs
                WHERE card_id = c.id AND repository IS NOT NULL
                ORDER BY created_at DESC
                LIMIT 1
            ) r ON TRUE
            WHERE c.organization_id = $1 AND c.project_id = $2 AND c."column" <> 'proposed'
            ORDER BY c.created_at DESC
            "#,
            organization_id.0,
            project_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| {
                let repository = row
                    .run_repository
                    .or(row.issue_repository)
                    .map(|name| name.parse())
                    .transpose()?;
                Ok(WorksiteSummary {
                    card_id: CardId(row.id),
                    title: row.title,
                    column: row.column,
                    issue_number: row.issue_number.map(|number| number as u64),
                    issue_url: row.issue_url,
                    repository,
                    run_id: row.run_id.map(RunId),
                    run_status: row.run_status,
                    created_at: row.created_at,
                })
            })
            .collect()
    }
}
