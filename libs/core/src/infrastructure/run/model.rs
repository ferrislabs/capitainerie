use capitainerie_common::{
    CardId, ClientId, CoreError, OrganizationId, ProjectId, RunId, StepId, TranscriptId,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    agent::Usage,
    client::{ClientRef, links::Links},
    project::ProjectRef,
    run::{
        Run, RunOverview, RunStatus, RunSubject, StepKind,
        step::{Step, StepStatus},
    },
};

pub(super) struct RunOverviewRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub transcript_id: Option<Uuid>,
    pub card_id: Option<Uuid>,
    pub repository: Option<String>,
    pub base_branch: Option<String>,
    pub initiated_by: String,
    pub status: String,
    pub pipeline: Vec<String>,
    pub failure: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub trace_id: Option<String>,
    pub models: Option<serde_json::Value>,
    pub title: Option<String>,
    pub repository_hint: Option<String>,
    pub client_id: Option<Uuid>,
    pub client_name: Option<String>,
    pub project_id: Option<Uuid>,
    pub project_name: Option<String>,
}

fn subject(row: &RunOverviewRow) -> Result<RunSubject, CoreError> {
    match (
        row.transcript_id,
        row.card_id,
        &row.repository,
        &row.base_branch,
    ) {
        (Some(transcript_id), None, None, None) => {
            Ok(RunSubject::Transcript(TranscriptId(transcript_id)))
        }
        (None, Some(card_id), Some(repository), Some(base_branch)) => Ok(RunSubject::Card {
            card_id: CardId(card_id),
            repository: repository.parse()?,
            base_branch: base_branch.clone(),
        }),
        (None, Some(card_id), None, None) => Ok(RunSubject::Intervention {
            card_id: CardId(card_id),
        }),
        _ => Err(CoreError::Internal(format!(
            "run {} has no valid subject",
            row.id
        ))),
    }
}

impl TryFrom<RunOverviewRow> for RunOverview {
    type Error = CoreError;

    fn try_from(row: RunOverviewRow) -> Result<Self, Self::Error> {
        let subject = subject(&row)?;
        Ok(Self {
            run: Run {
                id: RunId(row.id),
                organization_id: OrganizationId(row.organization_id),
                subject,
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
                models: row
                    .models
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(|error| CoreError::Internal(format!("stored run models: {error}")))?,
            },
            title: row.title,
            repository_hint: row
                .repository_hint
                .map(|value| value.parse())
                .transpose()
                .map_err(|error| CoreError::Internal(format!("stored repository hint: {error}")))?,
            links: Links::stored(
                row.client_id
                    .zip(row.client_name)
                    .map(|(id, name)| ClientRef {
                        id: ClientId(id),
                        name,
                    }),
                row.project_id
                    .zip(row.project_name)
                    .map(|(id, name)| ProjectRef {
                        id: ProjectId(id),
                        name,
                    }),
            ),
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
