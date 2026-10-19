use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::{ClientId, ProjectId, RunId, TranscriptId};
use capitainerie_core::{
    AgentDelegation,
    domain::{
        client::links::LinkChoice, run::service::SubmitTranscript, transcript::MeetingMetadata,
    },
};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(TypedPath)]
#[typed_path("/api/v1/transcripts")]
pub struct TranscriptsPath;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitTranscriptRequest {
    pub title: String,
    pub body: String,
    pub meeting_date: Option<NaiveDate>,
    pub participants: Option<Vec<String>>,
    pub repository: Option<String>,
    pub client_id: Option<ClientId>,
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitTranscriptResponse {
    pub transcript_id: TranscriptId,
    pub run_id: RunId,
}

#[utoipa::path(
    post,
    path = "/api/v1/transcripts",
    operation_id = "submitTranscript",
    tag = super::TAG,
    request_body = SubmitTranscriptRequest,
    responses(
        (status = 202, description = "Stored; a run is queued on it", body = SubmitTranscriptResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Not allowed", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Empty or oversized title or transcript, invalid participants or repository, unknown or archived client or project, or a client that is not the project's", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: TranscriptsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<SubmitTranscriptRequest>,
) -> Result<(StatusCode, Json<SubmitTranscriptResponse>), ApiError> {
    let metadata = MeetingMetadata::new(
        request.meeting_date,
        request.participants,
        request.repository,
    )?;
    let command = SubmitTranscript {
        subject: caller.subject(),
        title: request.title,
        body: request.body,
        metadata,
        choice: LinkChoice {
            client_id: request.client_id,
            project_id: request.project_id,
        },
    };
    let submitted = match &state.delegation {
        Some(delegation) => {
            state
                .usecase
                .submit_transcript_delegated(
                    command,
                    AgentDelegation {
                        exchanger: &delegation.exchanger,
                        issuer: &caller.organization.issuer_url,
                        user_token: &caller.token,
                        agents: &delegation.agents,
                    },
                )
                .await?
        }
        None => state.usecase.submit_transcript(command).await?,
    };
    Ok((
        StatusCode::ACCEPTED,
        Json(SubmitTranscriptResponse {
            transcript_id: submitted.transcript_id,
            run_id: submitted.run_id,
        }),
    ))
}
