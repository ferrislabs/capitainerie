use capitainerie_common::{RunId, TranscriptId};
use capitainerie_core::domain::run::RunOverview;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatusResponse {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunResponse {
    pub id: RunId,
    pub status: RunStatusResponse,
    pub transcript_id: TranscriptId,
    pub transcript_title: String,
    pub initiated_by: String,
    pub pipeline: Vec<String>,
    pub failure: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Trace id of the run in the trace backend.
    pub trace_id: Option<String>,
}

impl From<RunOverview> for RunResponse {
    fn from(
        RunOverview {
            run,
            transcript_title,
        }: RunOverview,
    ) -> Self {
        use capitainerie_core::domain::run::RunStatus;
        Self {
            id: run.id,
            status: match run.status {
                RunStatus::Queued => RunStatusResponse::Queued,
                RunStatus::Running => RunStatusResponse::Running,
                RunStatus::Succeeded => RunStatusResponse::Succeeded,
                RunStatus::Failed => RunStatusResponse::Failed,
                RunStatus::Cancelled => RunStatusResponse::Cancelled,
            },
            transcript_id: run.transcript_id,
            transcript_title,
            initiated_by: run.initiated_by,
            pipeline: run
                .pipeline
                .iter()
                .map(|step| step.as_str().to_owned())
                .collect(),
            failure: run.failure,
            created_at: run.created_at,
            started_at: run.started_at,
            finished_at: run.finished_at,
            trace_id: run.trace_id,
        }
    }
}
