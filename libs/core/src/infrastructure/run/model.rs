use capitainerie_common::{CoreError, OrganizationId, RunId, StepId, TranscriptId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    agent::Usage,
    run::{
        Run, RunOverview, RunStatus, StepKind,
        step::{Step, StepStatus},
    },
};

pub(super) struct RunOverviewRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub transcript_id: Uuid,
    pub initiated_by: String,
    pub status: String,
    pub pipeline: Vec<String>,
    pub failure: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub trace_id: Option<String>,
    pub transcript_title: String,
}

impl TryFrom<RunOverviewRow> for RunOverview {
    type Error = CoreError;

    fn try_from(row: RunOverviewRow) -> Result<Self, Self::Error> {
        Ok(Self {
            run: Run {
                id: RunId(row.id),
                organization_id: OrganizationId(row.organization_id),
                transcript_id: TranscriptId(row.transcript_id),
                initiated_by: row.initiated_by,
                status: RunStatus::try_from(row.status.as_str())?,
                pipeline: row
                    .pipeline
                    .iter()
                    .map(|step| StepKind::try_from(step.as_str()))
                    .collect::<Result<_, _>>()?,
                failure: row.failure,
                created_at: row.created_at,
                started_at: row.started_at,
                finished_at: row.finished_at,
                trace_id: row.trace_id,
            },
            transcript_title: row.transcript_title,
        })
    }
}

pub(super) struct StepRow {
    pub id: Uuid,
    pub run_id: Uuid,
    pub organization_id: Uuid,
    pub seq: i32,
    pub kind: String,
    pub agent: Option<String>,
    pub status: String,
    pub attempt: i32,
    pub output: Option<serde_json::Value>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl TryFrom<StepRow> for Step {
    type Error = CoreError;

    fn try_from(row: StepRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: StepId(row.id),
            run_id: RunId(row.run_id),
            organization_id: OrganizationId(row.organization_id),
            seq: row.seq,
            kind: StepKind::try_from(row.kind.as_str())?,
            agent: row.agent,
            status: StepStatus::try_from(row.status.as_str())?,
            attempt: row.attempt,
            output: row.output,
            usage: Usage {
                model: row.model.unwrap_or_default(),
                input_tokens: u64::try_from(row.input_tokens).unwrap_or_default(),
                output_tokens: u64::try_from(row.output_tokens).unwrap_or_default(),
            },
            cost_micros: row.cost_micros,
            latency_ms: row.latency_ms,
            error: row.error,
            started_at: row.started_at,
            finished_at: row.finished_at,
        })
    }
}
