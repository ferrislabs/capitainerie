use capitainerie_common::{CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;

use super::model::RunOverviewRow;
use crate::{
    domain::run::{Run, RunOverview, ports::RunRepository},
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Run, backend = Postgres)]
pub struct PgRunRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgRunRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl RunRepository for PgRunRepository<'_> {
    async fn insert(&mut self, run: &Run) -> Result<(), CoreError> {
        let pipeline: Vec<String> = run
            .pipeline
            .iter()
            .map(|step| step.as_str().to_owned())
            .collect();
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO runs (id, organization_id, transcript_id, initiated_by, status, pipeline, failure, created_at, started_at, finished_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            run.id.0,
            run.organization_id.0,
            run.transcript_id.0,
            run.initiated_by,
            run.status.as_str(),
            &pipeline,
            run.failure,
            run.created_at,
            run.started_at,
            run.finished_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<RunOverview>, u64), CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query_as!(
            RunOverviewRow,
            r#"
            SELECT r.id, r.organization_id, r.transcript_id, r.initiated_by, r.status, r.pipeline,
                   r.failure, r.created_at, r.started_at, r.finished_at, t.title AS transcript_title
            FROM runs r
            JOIN transcripts t ON t.id = r.transcript_id
            WHERE r.organization_id = $1
            ORDER BY r.created_at DESC, r.id DESC
            LIMIT $2 OFFSET $3
            "#,
            organization_id.0,
            i64::from(limit),
            i64::from(offset),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        let total = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM runs WHERE organization_id = $1"#,
            organization_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;

        let runs = rows
            .into_iter()
            .map(RunOverview::try_from)
            .collect::<Result<_, _>>()?;
        Ok((runs, u64::try_from(total).unwrap_or_default()))
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: RunId,
    ) -> Result<Option<RunOverview>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RunOverviewRow,
            r#"
            SELECT r.id, r.organization_id, r.transcript_id, r.initiated_by, r.status, r.pipeline,
                   r.failure, r.created_at, r.started_at, r.finished_at, t.title AS transcript_title
            FROM runs r
            JOIN transcripts t ON t.id = r.transcript_id
            WHERE r.organization_id = $1 AND r.id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(RunOverview::try_from)
        .transpose()
    }
}
