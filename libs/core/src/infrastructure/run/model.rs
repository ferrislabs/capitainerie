use capitainerie_common::{CoreError, OrganizationId, RunId, TranscriptId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::run::{Run, RunOverview, RunStatus, StepKind};

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
            },
            transcript_title: row.transcript_title,
        })
    }
}
