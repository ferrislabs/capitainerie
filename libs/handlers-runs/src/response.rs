use capitainerie_common::{CardId, ClientId, ProjectId, RunId, TranscriptId};
use capitainerie_core::domain::{
    client::links::Links,
    run::{RunOverview, RunSubject},
};
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ClientRefResponse {
    pub id: ClientId,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProjectRefResponse {
    pub id: ProjectId,
    pub name: String,
}

pub fn link_refs(links: &Links) -> (Option<ClientRefResponse>, Option<ProjectRefResponse>) {
    (
        links.client().map(|client| ClientRefResponse {
            id: client.id,
            name: client.name.clone(),
        }),
        links.project().map(|project| ProjectRefResponse {
            id: project.id,
            name: project.name.clone(),
        }),
    )
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunResponse {
    pub id: RunId,
    pub status: RunStatusResponse,
    pub transcript_id: Option<TranscriptId>,
    pub transcript_title: Option<String>,
    pub title: Option<String>,
    pub card_id: Option<CardId>,
    pub repository: Option<String>,
    pub repository_hint: Option<String>,
    pub client: Option<ClientRefResponse>,
    pub project: Option<ProjectRefResponse>,
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
            title,
            repository_hint,
            links,
        }: RunOverview,
    ) -> Self {
        use capitainerie_core::domain::run::RunStatus;
        let (transcript_id, card_id, repository) = match &run.subject {
            RunSubject::Transcript(id) => (Some(*id), None, None),
            RunSubject::Card {
                card_id,
                repository,
                ..
            } => (None, Some(*card_id), Some(repository.to_string())),
        };
        let transcript_title = transcript_id.and(title.clone());
        let (client, project) = link_refs(&links);
        Self {
            id: run.id,
            status: match run.status {
                RunStatus::Queued => RunStatusResponse::Queued,
                RunStatus::Running => RunStatusResponse::Running,
                RunStatus::Succeeded => RunStatusResponse::Succeeded,
                RunStatus::Failed => RunStatusResponse::Failed,
                RunStatus::Cancelled => RunStatusResponse::Cancelled,
            },
            transcript_id,
            transcript_title,
            title,
            card_id,
            repository,
            repository_hint: repository_hint.map(|hint| hint.to_string()),
            client,
            project,
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
