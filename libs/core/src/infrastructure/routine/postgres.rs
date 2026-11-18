use std::collections::HashMap;

use capitainerie_common::{
    AgentId, CardId, CoreError, OrganizationId, ProjectId, RoutineId, RoutineRunId,
    RoutineTriggerId, RunId,
};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        card::{Column, attributes::Priority},
        crew::Slug,
        routine::{
            IfMissed, IfOpen, Routine, RoutineListing, RoutineRun, RoutineRunStatus,
            RoutineRunView, RunCounts, RunOrigin, TitleTemplate, Trigger, TriggerKind,
            ports::{RoutineRepository, RoutineRunRepository},
            schedule::{Schedule, ScheduleSpec, parse_timezone},
        },
        run::RunStatus,
        source::TokenHash,
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const LIST_LIMIT: i64 = 500;

#[repository(domain = Routine, backend = Postgres)]
#[derive(Clone)]
pub struct PgRoutineRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgRoutineRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }

    async fn listings(&mut self, routines: Vec<Routine>) -> Result<Vec<RoutineListing>, CoreError> {
        let ids: Vec<Uuid> = routines.iter().map(|routine| routine.id.0).collect();
        let mut tx = self.tx.lock().await;
        let mut triggers: HashMap<Uuid, Vec<Trigger>> = HashMap::new();
        for row in sqlx::query_as!(
            TriggerRow,
            r#"
            SELECT id, organization_id, routine_id, kind, enabled, frequency, every_hours,
                   at_time, weekdays, cron, next_run_at, webhook_token_hash, webhook_token_hint,
                   created_at
            FROM routine_triggers
            WHERE routine_id = ANY($1)
            ORDER BY created_at, id
            "#,
            &ids,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        {
            let routine_id = row.routine_id;
            triggers
                .entry(routine_id)
                .or_default()
                .push(Trigger::try_from(row)?);
        }
        let mut last_runs: HashMap<Uuid, RoutineRunView> = HashMap::new();
        for row in sqlx::query_as!(
            RunViewRow,
            r#"
            SELECT DISTINCT ON (rr.routine_id)
                   rr.id, rr.organization_id, rr.routine_id, rr.trigger_id, rr.origin,
                   rr.scheduled_for, rr.started_at, rr.status, rr.card_id, rr.run_id, rr.reason,
                   c."column" AS "card_column?", r.status AS "run_status?",
                   r.started_at AS run_started_at, r.finished_at AS run_finished_at
            FROM routine_runs rr
            LEFT JOIN cards c ON c.id = rr.card_id
            LEFT JOIN runs r ON r.id = rr.run_id
            WHERE rr.routine_id = ANY($1)
            ORDER BY rr.routine_id, rr.started_at DESC, rr.id DESC
            "#,
            &ids,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        {
            let routine_id = row.routine_id;
            last_runs.insert(routine_id, RoutineRunView::try_from(row)?);
        }
        let counts: HashMap<Uuid, RunCounts> = sqlx::query!(
            r#"
            SELECT routine_id,
                   count(*) FILTER (WHERE status = 'created') AS "created!",
                   count(*) FILTER (WHERE status = 'skipped') AS "skipped!",
                   count(*) FILTER (WHERE status = 'failed') AS "failed!"
            FROM routine_runs
            WHERE routine_id = ANY($1)
            GROUP BY routine_id
            "#,
            &ids,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(|row| {
            (
                row.routine_id,
                RunCounts {
                    created: row.created,
                    skipped: row.skipped,
                    failed: row.failed,
                },
            )
        })
        .collect();
        Ok(routines
            .into_iter()
            .map(|routine| RoutineListing {
                triggers: triggers.remove(&routine.id.0).unwrap_or_default(),
                last_run: last_runs.remove(&routine.id.0),
                counts: counts.get(&routine.id.0).copied().unwrap_or_default(),
                routine,
            })
            .collect())
    }
}

impl RoutineRepository for PgRoutineRepository<'_> {
    async fn insert(&mut self, routine: &Routine) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO routines (
                id, organization_id, slug, name, prompt, agent_id, project_id, priority,
                title_template, timezone, enabled, if_open, if_missed, allow_writes,
                last_run_at, archived, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                    $17, $18, $19)
            "#,
            routine.id.0,
            routine.organization_id.0,
            routine.slug.as_str(),
            routine.name,
            routine.prompt,
            routine.agent_id.0,
            routine.project_id.map(|id| id.0),
            routine.priority.as_str(),
            routine.title_template.as_str(),
            routine.timezone.name(),
            routine.enabled,
            routine.if_open.as_str(),
            routine.if_missed.as_str(),
            routine.allow_writes,
            routine.last_run_at,
            routine.archived,
            routine.created_by,
            routine.created_at,
            routine.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| named(error, routine))
    }

    async fn update(&mut self, routine: &Routine) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE routines
            SET slug = $3, name = $4, prompt = $5, agent_id = $6, project_id = $7,
                priority = $8, title_template = $9, timezone = $10, enabled = $11,
                if_open = $12, if_missed = $13, allow_writes = $14, archived = $15,
                updated_at = $16
            WHERE organization_id = $1 AND id = $2
            "#,
            routine.organization_id.0,
            routine.id.0,
            routine.slug.as_str(),
            routine.name,
            routine.prompt,
            routine.agent_id.0,
            routine.project_id.map(|id| id.0),
            routine.priority.as_str(),
            routine.title_template.as_str(),
            routine.timezone.name(),
            routine.enabled,
            routine.if_open.as_str(),
            routine.if_missed.as_str(),
            routine.allow_writes,
            routine.archived,
            routine.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(|error| named(error, routine))?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: RoutineId,
    ) -> Result<Option<RoutineListing>, CoreError> {
        let routine = {
            let mut tx = self.tx.lock().await;
            sqlx::query_as!(
                RoutineRow,
                r#"
                SELECT id, organization_id, slug, name, prompt, agent_id, project_id, priority,
                       title_template, timezone, enabled, if_open, if_missed, allow_writes,
                       last_run_at, archived, created_by, created_at, updated_at
                FROM routines
                WHERE organization_id = $1 AND id = $2
                "#,
                organization_id.0,
                id.0,
            )
            .fetch_optional(&mut ***tx)
            .await
            .map_err(map_sqlx_error)?
            .map(Routine::try_from)
            .transpose()?
        };
        match routine {
            None => Ok(None),
            Some(routine) => Ok(self.listings(vec![routine]).await?.pop()),
        }
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> Result<Vec<RoutineListing>, CoreError> {
        let routines = {
            let mut tx = self.tx.lock().await;
            sqlx::query_as!(
                RoutineRow,
                r#"
                SELECT id, organization_id, slug, name, prompt, agent_id, project_id, priority,
                       title_template, timezone, enabled, if_open, if_missed, allow_writes,
                       last_run_at, archived, created_by, created_at, updated_at
                FROM routines
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
            .map(Routine::try_from)
            .collect::<Result<Vec<_>, _>>()?
        };
        self.listings(routines).await
    }

    async fn insert_trigger(&mut self, trigger: &Trigger) -> Result<(), CoreError> {
        let columns = TriggerColumns::of(trigger);
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO routine_triggers (
                id, organization_id, routine_id, kind, enabled, frequency, every_hours,
                at_time, weekdays, cron, next_run_at, webhook_token_hash, webhook_token_hint,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
            trigger.id.0,
            trigger.organization_id.0,
            trigger.routine_id.0,
            trigger.kind.as_str(),
            trigger.enabled,
            columns.frequency,
            columns.every_hours,
            columns.at_time,
            &columns.weekdays,
            columns.cron,
            trigger.next_run_at,
            columns.token_hash,
            columns.token_hint,
            trigger.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn update_trigger(&mut self, trigger: &Trigger) -> Result<(), CoreError> {
        let columns = TriggerColumns::of(trigger);
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE routine_triggers
            SET enabled = $3, frequency = $4, every_hours = $5, at_time = $6, weekdays = $7,
                cron = $8, next_run_at = $9, webhook_token_hash = $10, webhook_token_hint = $11
            WHERE organization_id = $1 AND id = $2
            "#,
            trigger.organization_id.0,
            trigger.id.0,
            trigger.enabled,
            columns.frequency,
            columns.every_hours,
            columns.at_time,
            &columns.weekdays,
            columns.cron,
            trigger.next_run_at,
            columns.token_hash,
            columns.token_hint,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn delete_trigger(
        &mut self,
        organization_id: OrganizationId,
        routine_id: RoutineId,
        id: RoutineTriggerId,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            DELETE FROM routine_triggers
            WHERE organization_id = $1 AND routine_id = $2 AND id = $3
            "#,
            organization_id.0,
            routine_id.0,
            id.0,
        )
        .execute(&mut ***tx)
        .await
        .map(|done| done.rows_affected() > 0)
        .map_err(map_sqlx_error)
    }

    async fn find_by_token(
        &mut self,
        hash: &TokenHash,
    ) -> Result<Option<(Routine, Trigger)>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ClaimRow,
            r#"
            SELECT r.id, r.organization_id, r.slug, r.name, r.prompt, r.agent_id, r.project_id,
                   r.priority, r.title_template, r.timezone, r.enabled, r.if_open, r.if_missed,
                   r.allow_writes, r.last_run_at, r.archived, r.created_by, r.created_at,
                   r.updated_at,
                   t.id AS trigger_id, t.kind, t.enabled AS trigger_enabled, t.frequency,
                   t.every_hours, t.at_time, t.weekdays, t.cron, t.next_run_at,
                   t.webhook_token_hash, t.webhook_token_hint, t.created_at AS trigger_created_at
            FROM routine_triggers t
            JOIN routines r ON r.id = t.routine_id
            WHERE t.webhook_token_hash = $1
            FOR UPDATE OF t, r
            "#,
            hash.as_str(),
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(ClaimRow::split)
        .transpose()
    }

    async fn claim_due(
        &mut self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<(Routine, Trigger)>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ClaimRow,
            r#"
            SELECT r.id, r.organization_id, r.slug, r.name, r.prompt, r.agent_id, r.project_id,
                   r.priority, r.title_template, r.timezone, r.enabled, r.if_open, r.if_missed,
                   r.allow_writes, r.last_run_at, r.archived, r.created_by, r.created_at,
                   r.updated_at,
                   t.id AS trigger_id, t.kind, t.enabled AS trigger_enabled, t.frequency,
                   t.every_hours, t.at_time, t.weekdays, t.cron, t.next_run_at,
                   t.webhook_token_hash, t.webhook_token_hint, t.created_at AS trigger_created_at
            FROM routine_triggers t
            JOIN routines r ON r.id = t.routine_id
            WHERE t.enabled AND r.enabled AND NOT r.archived
              AND (t.next_run_at <= $1
                   OR EXISTS (SELECT 1 FROM routine_runs q
                              WHERE q.trigger_id = t.id AND q.status = 'queued'))
              -- A queued passage whose previous card is still open keeps waiting.
              AND NOT (
                  EXISTS (SELECT 1 FROM routine_runs q
                          WHERE q.trigger_id = t.id AND q.status = 'queued')
                  AND COALESCE(
                      (SELECT c."column" FROM routine_runs p
                       JOIN cards c ON c.id = p.card_id
                       WHERE p.routine_id = r.id
                       ORDER BY p.started_at DESC, p.id DESC
                       LIMIT 1),
                      'done'
                  ) NOT IN ('done', 'rejected')
              )
            ORDER BY t.next_run_at NULLS LAST, t.id
            LIMIT $2
            FOR UPDATE OF t, r SKIP LOCKED
            "#,
            now,
            limit,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(ClaimRow::split)
        .collect()
    }

    async fn mark_ran(
        &mut self,
        organization_id: OrganizationId,
        id: RoutineId,
        at: DateTime<Utc>,
    ) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "UPDATE routines SET last_run_at = $3 WHERE organization_id = $1 AND id = $2",
            organization_id.0,
            id.0,
            at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}

fn named(error: sqlx::Error, routine: &Routine) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.constraint() == Some("routines_slug_unique") => {
            CoreError::Conflict(format!(
                "a routine with the slug {} already exists",
                routine.slug.as_str()
            ))
        }
        other => map_sqlx_error(other),
    }
}

struct RoutineRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    prompt: String,
    agent_id: Uuid,
    project_id: Option<Uuid>,
    priority: String,
    title_template: String,
    timezone: String,
    enabled: bool,
    if_open: String,
    if_missed: String,
    allow_writes: bool,
    last_run_at: Option<DateTime<Utc>>,
    archived: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<RoutineRow> for Routine {
    type Error = CoreError;

    fn try_from(row: RoutineRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored routine {}: {error}", row.id));
        Ok(Self {
            id: RoutineId(row.id),
            organization_id: OrganizationId(row.organization_id),
            slug: Slug::try_from(row.slug).map_err(stored)?,
            name: row.name,
            prompt: row.prompt,
            agent_id: AgentId(row.agent_id),
            project_id: row.project_id.map(ProjectId),
            priority: Priority::try_from(row.priority.as_str()).map_err(stored)?,
            title_template: TitleTemplate::try_from(row.title_template).map_err(stored)?,
            timezone: parse_timezone(&row.timezone).map_err(stored)?,
            enabled: row.enabled,
            if_open: IfOpen::try_from(row.if_open.as_str()).map_err(stored)?,
            if_missed: IfMissed::try_from(row.if_missed.as_str()).map_err(stored)?,
            allow_writes: row.allow_writes,
            last_run_at: row.last_run_at,
            archived: row.archived,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

struct TriggerColumns {
    frequency: Option<String>,
    every_hours: Option<i16>,
    at_time: Option<String>,
    weekdays: Vec<String>,
    cron: Option<String>,
    token_hash: Option<String>,
    token_hint: Option<String>,
}

impl TriggerColumns {
    fn of(trigger: &Trigger) -> Self {
        match &trigger.kind {
            TriggerKind::Planning(schedule) => {
                let spec = schedule.spec();
                Self {
                    frequency: Some(spec.frequency),
                    every_hours: spec.hours.and_then(|hours| i16::try_from(hours).ok()),
                    at_time: spec.time,
                    weekdays: spec.days,
                    cron: spec.cron,
                    token_hash: None,
                    token_hint: None,
                }
            }
            TriggerKind::Webhook {
                token_hash,
                token_hint,
            } => Self {
                frequency: None,
                every_hours: None,
                at_time: None,
                weekdays: Vec::new(),
                cron: None,
                token_hash: Some(token_hash.as_str().to_owned()),
                token_hint: Some(token_hint.clone()),
            },
        }
    }
}

struct TriggerRow {
    id: Uuid,
    organization_id: Uuid,
    routine_id: Uuid,
    kind: String,
    enabled: bool,
    frequency: Option<String>,
    every_hours: Option<i16>,
    at_time: Option<String>,
    weekdays: Vec<String>,
    cron: Option<String>,
    next_run_at: Option<DateTime<Utc>>,
    webhook_token_hash: Option<String>,
    webhook_token_hint: Option<String>,
    created_at: DateTime<Utc>,
}

impl TryFrom<TriggerRow> for Trigger {
    type Error = CoreError;

    fn try_from(row: TriggerRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored trigger {}: {error}", row.id));
        let kind = match (row.kind.as_str(), row.frequency, row.webhook_token_hash) {
            ("planning", Some(frequency), None) => TriggerKind::Planning(
                Schedule::try_from(ScheduleSpec {
                    frequency,
                    hours: row.every_hours.and_then(|hours| u16::try_from(hours).ok()),
                    time: row.at_time,
                    days: row.weekdays,
                    cron: row.cron,
                })
                .map_err(stored)?,
            ),
            ("webhook", None, Some(hash)) => TriggerKind::Webhook {
                token_hash: TokenHash::from_stored(hash).map_err(stored)?,
                token_hint: row.webhook_token_hint.unwrap_or_default(),
            },
            _ => {
                return Err(CoreError::Internal(format!(
                    "stored trigger {} is malformed",
                    row.id
                )));
            }
        };
        Ok(Self {
            id: RoutineTriggerId(row.id),
            organization_id: OrganizationId(row.organization_id),
            routine_id: RoutineId(row.routine_id),
            enabled: row.enabled,
            kind,
            next_run_at: row.next_run_at,
            created_at: row.created_at,
        })
    }
}

struct ClaimRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    prompt: String,
    agent_id: Uuid,
    project_id: Option<Uuid>,
    priority: String,
    title_template: String,
    timezone: String,
    enabled: bool,
    if_open: String,
    if_missed: String,
    allow_writes: bool,
    last_run_at: Option<DateTime<Utc>>,
    archived: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    trigger_id: Uuid,
    kind: String,
    trigger_enabled: bool,
    frequency: Option<String>,
    every_hours: Option<i16>,
    at_time: Option<String>,
    weekdays: Vec<String>,
    cron: Option<String>,
    next_run_at: Option<DateTime<Utc>>,
    webhook_token_hash: Option<String>,
    webhook_token_hint: Option<String>,
    trigger_created_at: DateTime<Utc>,
}

impl ClaimRow {
    fn split(self) -> Result<(Routine, Trigger), CoreError> {
        let trigger = Trigger::try_from(TriggerRow {
            id: self.trigger_id,
            organization_id: self.organization_id,
            routine_id: self.id,
            kind: self.kind,
            enabled: self.trigger_enabled,
            frequency: self.frequency,
            every_hours: self.every_hours,
            at_time: self.at_time,
            weekdays: self.weekdays,
            cron: self.cron,
            next_run_at: self.next_run_at,
            webhook_token_hash: self.webhook_token_hash,
            webhook_token_hint: self.webhook_token_hint,
            created_at: self.trigger_created_at,
        })?;
        let routine = Routine::try_from(RoutineRow {
            id: self.id,
            organization_id: self.organization_id,
            slug: self.slug,
            name: self.name,
            prompt: self.prompt,
            agent_id: self.agent_id,
            project_id: self.project_id,
            priority: self.priority,
            title_template: self.title_template,
            timezone: self.timezone,
            enabled: self.enabled,
            if_open: self.if_open,
            if_missed: self.if_missed,
            allow_writes: self.allow_writes,
            last_run_at: self.last_run_at,
            archived: self.archived,
            created_by: self.created_by,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })?;
        Ok((routine, trigger))
    }
}

#[repository(domain = RoutineRun, backend = Postgres)]
pub struct PgRoutineRunRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgRoutineRunRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl RoutineRunRepository for PgRoutineRunRepository<'_> {
    async fn insert(&mut self, run: &RoutineRun) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO routine_runs (
                id, organization_id, routine_id, trigger_id, origin, scheduled_for,
                started_at, status, card_id, run_id, reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            run.id.0,
            run.organization_id.0,
            run.routine_id.0,
            run.trigger_id.map(|id| id.0),
            run.origin.as_str(),
            run.scheduled_for,
            run.started_at,
            run.status.as_str(),
            run.card_id.map(|id| id.0),
            run.run_id.map(|id| id.0),
            run.reason,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn save(&mut self, run: &RoutineRun) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE routine_runs
            SET started_at = $3, status = $4, card_id = $5, run_id = $6, reason = $7
            WHERE organization_id = $1 AND id = $2
            "#,
            run.organization_id.0,
            run.id.0,
            run.started_at,
            run.status.as_str(),
            run.card_id.map(|id| id.0),
            run.run_id.map(|id| id.0),
            run.reason,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn queued(
        &mut self,
        trigger_id: RoutineTriggerId,
    ) -> Result<Option<RoutineRun>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RunRow,
            r#"
            SELECT id, organization_id, routine_id, trigger_id, origin, scheduled_for,
                   started_at, status, card_id, run_id, reason
            FROM routine_runs
            WHERE trigger_id = $1 AND status = 'queued'
            ORDER BY started_at, id
            LIMIT 1
            "#,
            trigger_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(RoutineRun::try_from)
        .transpose()
    }

    async fn previous_card_open(&mut self, routine_id: RoutineId) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        let column = sqlx::query_scalar!(
            r#"
            SELECT c."column"
            FROM routine_runs rr
            JOIN cards c ON c.id = rr.card_id
            WHERE rr.routine_id = $1
            ORDER BY rr.started_at DESC, rr.id DESC
            LIMIT 1
            "#,
            routine_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(column.is_some_and(|column| column != "done" && column != "rejected"))
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        routine_id: RoutineId,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RoutineRunView>, i64), CoreError> {
        let mut tx = self.tx.lock().await;
        let total = sqlx::query_scalar!(
            r#"
            SELECT count(*) AS "count!" FROM routine_runs
            WHERE organization_id = $1 AND routine_id = $2
            "#,
            organization_id.0,
            routine_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        let runs = sqlx::query_as!(
            RunViewRow,
            r#"
            SELECT rr.id, rr.organization_id, rr.routine_id, rr.trigger_id, rr.origin,
                   rr.scheduled_for, rr.started_at, rr.status, rr.card_id, rr.run_id, rr.reason,
                   c."column" AS "card_column?", r.status AS "run_status?",
                   r.started_at AS run_started_at, r.finished_at AS run_finished_at
            FROM routine_runs rr
            LEFT JOIN cards c ON c.id = rr.card_id
            LEFT JOIN runs r ON r.id = rr.run_id
            WHERE rr.organization_id = $1 AND rr.routine_id = $2
            ORDER BY rr.started_at DESC, rr.id DESC
            LIMIT $3 OFFSET $4
            "#,
            organization_id.0,
            routine_id.0,
            limit,
            offset,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(RoutineRunView::try_from)
        .collect::<Result<Vec<_>, _>>()?;
        Ok((runs, total))
    }

    async fn fail_stale(&mut self, before: DateTime<Utc>, reason: &str) -> Result<u64, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            UPDATE routine_runs SET status = 'failed', reason = $2
            WHERE status = 'pending' AND started_at < $1
            "#,
            before,
            reason,
        )
        .execute(&mut ***tx)
        .await
        .map(|done| done.rows_affected())
        .map_err(map_sqlx_error)
    }
}

struct RunRow {
    id: Uuid,
    organization_id: Uuid,
    routine_id: Uuid,
    trigger_id: Option<Uuid>,
    origin: String,
    scheduled_for: DateTime<Utc>,
    started_at: DateTime<Utc>,
    status: String,
    card_id: Option<Uuid>,
    run_id: Option<Uuid>,
    reason: Option<String>,
}

impl TryFrom<RunRow> for RoutineRun {
    type Error = CoreError;

    fn try_from(row: RunRow) -> Result<Self, Self::Error> {
        let stored = |error: CoreError| {
            CoreError::Internal(format!("stored routine run {}: {error}", row.id))
        };
        Ok(Self {
            id: RoutineRunId(row.id),
            organization_id: OrganizationId(row.organization_id),
            routine_id: RoutineId(row.routine_id),
            trigger_id: row.trigger_id.map(RoutineTriggerId),
            origin: RunOrigin::try_from(row.origin.as_str()).map_err(stored)?,
            scheduled_for: row.scheduled_for,
            started_at: row.started_at,
            status: RoutineRunStatus::try_from(row.status.as_str()).map_err(stored)?,
            card_id: row.card_id.map(CardId),
            run_id: row.run_id.map(RunId),
            reason: row.reason,
        })
    }
}

struct RunViewRow {
    id: Uuid,
    organization_id: Uuid,
    routine_id: Uuid,
    trigger_id: Option<Uuid>,
    origin: String,
    scheduled_for: DateTime<Utc>,
    started_at: DateTime<Utc>,
    status: String,
    card_id: Option<Uuid>,
    run_id: Option<Uuid>,
    reason: Option<String>,
    card_column: Option<String>,
    run_status: Option<String>,
    run_started_at: Option<DateTime<Utc>>,
    run_finished_at: Option<DateTime<Utc>>,
}

impl TryFrom<RunViewRow> for RoutineRunView {
    type Error = CoreError;

    fn try_from(row: RunViewRow) -> Result<Self, Self::Error> {
        let stored = |error: CoreError| {
            CoreError::Internal(format!("stored routine run {}: {error}", row.id))
        };
        let card_column = row
            .card_column
            .as_deref()
            .map(Column::try_from)
            .transpose()
            .map_err(stored)?;
        let agent_run_status = row
            .run_status
            .as_deref()
            .map(RunStatus::try_from)
            .transpose()
            .map_err(stored)?;
        Ok(Self {
            run: RoutineRun::try_from(RunRow {
                id: row.id,
                organization_id: row.organization_id,
                routine_id: row.routine_id,
                trigger_id: row.trigger_id,
                origin: row.origin,
                scheduled_for: row.scheduled_for,
                started_at: row.started_at,
                status: row.status,
                card_id: row.card_id,
                run_id: row.run_id,
                reason: row.reason,
            })?,
            card_column,
            agent_run_status,
            agent_run_started_at: row.run_started_at,
            agent_run_finished_at: row.run_finished_at,
        })
    }
}
