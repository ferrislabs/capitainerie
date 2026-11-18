use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::AgentId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::{TeamAgentResponse, TeamAgentRevisionListResponse};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/team/agents/{agent_id}/revisions")]
pub struct TeamAgentRevisionsPath {
    pub agent_id: AgentId,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/team/agents/{agent_id}/revisions/{revision}/restore")]
pub struct RestoreTeamAgentRevisionPath {
    pub agent_id: AgentId,
    pub revision: i32,
}

#[utoipa::path(
    get,
    path = "/api/v1/team/agents/{agent_id}/revisions",
    operation_id = "listTeamAgentRevisions",
    tag = super::TAG,
    params(("agent_id" = AgentId, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "The agent's history, newest first", body = TeamAgentRevisionListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to the registry", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such agent in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_handler(
    TeamAgentRevisionsPath { agent_id }: TeamAgentRevisionsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<TeamAgentRevisionListResponse>, ApiError> {
    let revisions = state
        .usecase
        .list_crew_agent_revisions(caller.subject(), agent_id)
        .await?;
    Ok(Json(TeamAgentRevisionListResponse {
        items: revisions.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/team/agents/{agent_id}/revisions/{revision}/restore",
    operation_id = "restoreTeamAgentRevision",
    tag = super::TAG,
    params(
        ("agent_id" = AgentId, Path, description = "Agent identifier"),
        ("revision" = i32, Path, description = "The revision whose state comes back"),
    ),
    responses(
        (status = 200, description = "The agent with the revision's state, recorded as a new revision", body = TeamAgentResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage the registry", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such agent in the caller's organization, or no such revision", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The revision's manager would now close a reporting loop", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_handler(
    RestoreTeamAgentRevisionPath { agent_id, revision }: RestoreTeamAgentRevisionPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<TeamAgentResponse>, ApiError> {
    let agent = state
        .usecase
        .restore_crew_agent_revision(caller.subject(), agent_id, revision)
        .await?;
    Ok(Json(agent.into()))
}
