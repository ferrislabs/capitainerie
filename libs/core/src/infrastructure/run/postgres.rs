use capitainerie_common::{CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};

use super::model::{RunOverviewRow, StepRow};
use crate::{
    domain::run::{
        Run, RunOverview, RunSubject,
        ports::{RunRepository, StepRepository},
        step::Step,
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const ACTIVE_EXECUTION_INDEX: &str = "runs_one_active_execution";

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
        let models = run
            .models
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| CoreError::Internal(format!("encoding run models: {error}")))?;
        let (transcript_id, card_id, repository, base_branch) = match &run.subject {
            RunSubject::Transcript(id) => (Some(id.0), None, None, None),
            RunSubject::Card {
                card_id,
                repository,
                base_branch,
            } => (
                None,
                Some(card_id.0),
                Some(repository.to_string()),
                Some(base_branch.as_str()),
            ),
        };
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO runs (id, organization_id, transcript_id, card_id, repository, base_branch, initiated_by, status, pipeline, failure, created_at, started_at, finished_at, trace_id, models)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            run.id.0,
            run.organization_id.0,
            transcript_id,
            card_id,
            repository,
            base_branch,
            run.initiated_by,
            run.status.as_str(),
            &pipeline,
            run.failure,
            run.created_at,
            run.started_at,
            run.finished_at,
            run.trace_id,
            models,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| match &error {
            sqlx::Error::Database(db) if db.constraint() == Some(ACTIVE_EXECUTION_INDEX) => {
                CoreError::Conflict("execution_already_active".into())
            }
            _ => map_sqlx_error(error),
        })
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
            SELECT r.id, r.organization_id, r.transcript_id, r.card_id, r.repository, r.base_branch, r.initiated_by, r.status, r.pipeline,
                   r.failure, r.created_at, r.started_at, r.finished_at, r.trace_id, r.models, COALESCE(t.title, c.title) AS title
            FROM runs r
            LEFT JOIN transcripts t ON t.id = r.transcript_id
            LEFT JOIN cards c ON c.id = r.card_id
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
            SELECT r.id, r.organization_id, r.transcript_id, r.card_id, r.repository, r.base_branch, r.initiated_by, r.status, r.pipeline,
                   r.failure, r.created_at, r.started_at, r.finished_at, r.trace_id, r.models, COALESCE(t.title, c.title) AS title
            FROM runs r
            LEFT JOIN transcripts t ON t.id = r.transcript_id
            LEFT JOIN cards c ON c.id = r.card_id
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

    async fn list_for_card(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> Result<Vec<RunOverview>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RunOverviewRow,
            r#"
            SELECT r.id, r.organization_id, r.transcript_id, r.card_id, r.repository, r.base_branch, r.initiated_by, r.status, r.pipeline,
                   r.failure, r.created_at, r.started_at, r.finished_at, r.trace_id, r.models, COALESCE(t.title, c.title) AS title
            FROM runs r
            LEFT JOIN transcripts t ON t.id = r.transcript_id
            LEFT JOIN cards c ON c.id = r.card_id
            WHERE r.organization_id = $1 AND r.card_id = $2
            ORDER BY r.created_at DESC, r.id DESC
            "#,
            organization_id.0,
            card_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(RunOverview::try_from)
        .collect()
    }

    async fn list_active(
        &mut self,
        organization_id: OrganizationId,
        limit: u32,
    ) -> Result<Vec<RunOverview>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RunOverviewRow,
            r#"
            SELECT r.id, r.organization_id, r.transcript_id, r.card_id, r.repository, r.base_branch, r.initiated_by, r.status, r.pipeline,
                   r.failure, r.created_at, r.started_at, r.finished_at, r.trace_id, r.models, COALESCE(t.title, c.title) AS title
            FROM runs r
            LEFT JOIN transcripts t ON t.id = r.transcript_id
            LEFT JOIN cards c ON c.id = r.card_id
            WHERE r.organization_id = $1 AND r.status IN ('queued', 'running')
            ORDER BY r.created_at, r.id
            LIMIT $2
            "#,
            organization_id.0,
            i64::from(limit),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(RunOverview::try_from)
        .collect()
    }

    async fn lock(&mut self, id: RunId) -> Result<Option<Run>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RunOverviewRow,
            r#"
            SELECT r.id, r.organization_id, r.transcript_id, r.card_id, r.repository, r.base_branch, r.initiated_by, r.status, r.pipeline,
                   r.failure, r.created_at, r.started_at, r.finished_at, r.trace_id, r.models, COALESCE(t.title, c.title) AS title
            FROM runs r
            LEFT JOIN transcripts t ON t.id = r.transcript_id
            LEFT JOIN cards c ON c.id = r.card_id
            WHERE r.id = $1
            FOR UPDATE OF r SKIP LOCKED
            "#,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(|row| RunOverview::try_from(row).map(|overview| overview.run))
        .transpose()
    }

    async fn save(
        &mut self,
        run: &Run,
        heartbeat_at: Option<DateTime<Utc>>,
    ) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            UPDATE runs
            SET status = $2, failure = $3, started_at = $4, finished_at = $5, trace_id = $6,
                heartbeat_at = COALESCE($7, heartbeat_at)
            WHERE id = $1
            "#,
            run.id.0,
            run.status.as_str(),
            run.failure,
            run.started_at,
            run.finished_at,
            run.trace_id,
            heartbeat_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn pending(
        &mut self,
        stale_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<RunId>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"
            SELECT id
            FROM runs
            WHERE status = 'queued'
               OR (status = 'running' AND (heartbeat_at IS NULL OR heartbeat_at < $1))
            ORDER BY created_at
            LIMIT $2
            "#,
            stale_before,
            limit,
        )
        .fetch_all(&mut ***tx)
        .await
        .map(|ids| ids.into_iter().map(RunId).collect())
        .map_err(map_sqlx_error)
    }

    async fn has_active_execution(&mut self, card_id: CardId) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM runs WHERE card_id = $1 AND status IN ('queued', 'running')
            ) AS "active!"
            "#,
            card_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn heartbeat_at(&mut self, id: RunId) -> Result<Option<DateTime<Utc>>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!("SELECT heartbeat_at FROM runs WHERE id = $1", id.0)
            .fetch_optional(&mut ***tx)
            .await
            .map(Option::flatten)
            .map_err(map_sqlx_error)
    }
}

#[repository(domain = Step, backend = Postgres)]
pub struct PgStepRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgStepRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl StepRepository for PgStepRepository<'_> {
    async fn list(&mut self, run_id: RunId) -> Result<Vec<Step>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            StepRow,
            r#"
            SELECT id, run_id, organization_id, seq, kind, agent, status, attempt, output, model,
                   input_tokens, output_tokens, cost_micros, latency_ms, error, started_at, finished_at
            FROM run_steps
            WHERE run_id = $1
            ORDER BY seq
            "#,
            run_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Step::try_from)
        .collect()
    }

    async fn list_for_runs(&mut self, run_ids: &[RunId]) -> Result<Vec<Step>, CoreError> {
        let ids: Vec<uuid::Uuid> = run_ids.iter().map(|id| id.0).collect();
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            StepRow,
            r#"
            SELECT id, run_id, organization_id, seq, kind, agent, status, attempt, output, model,
                   input_tokens, output_tokens, cost_micros, latency_ms, error, started_at, finished_at
            FROM run_steps
            WHERE run_id = ANY($1)
            ORDER BY run_id, seq
            "#,
            &ids,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Step::try_from)
        .collect()
    }

    async fn save(&mut self, step: &Step) -> Result<(), CoreError> {
        let input_tokens = i64::try_from(step.usage.input_tokens).unwrap_or(i64::MAX);
        let output_tokens = i64::try_from(step.usage.output_tokens).unwrap_or(i64::MAX);
        let model = (!step.usage.model.is_empty()).then_some(step.usage.model.as_str());
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO run_steps (id, run_id, organization_id, seq, kind, agent, status, attempt, output, model,
                                   input_tokens, output_tokens, cost_micros, latency_ms, error, started_at, finished_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (run_id, seq) DO UPDATE
            SET id = EXCLUDED.id, agent = EXCLUDED.agent, status = EXCLUDED.status, attempt = EXCLUDED.attempt,
                output = EXCLUDED.output, model = EXCLUDED.model, input_tokens = EXCLUDED.input_tokens,
                output_tokens = EXCLUDED.output_tokens, cost_micros = EXCLUDED.cost_micros,
                latency_ms = EXCLUDED.latency_ms, error = EXCLUDED.error, started_at = EXCLUDED.started_at,
                finished_at = EXCLUDED.finished_at
            "#,
            step.id.0,
            step.run_id.0,
            step.organization_id.0,
            step.seq,
            step.kind.as_str(),
            step.agent,
            step.status.as_str(),
            step.attempt,
            step.output,
            model,
            input_tokens,
            output_tokens,
            step.cost_micros,
            step.latency_ms,
            step.error,
            step.started_at,
            step.finished_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}
