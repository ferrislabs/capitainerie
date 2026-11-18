use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::AgentId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::TeamAgentResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/team/agents/{agent_id}")]
pub struct TeamAgentPath {
    pub agent_id: AgentId,
}

#[utoipa::path(
    get,
    path = "/api/v1/team/agents/{agent_id}",
    operation_id = "getTeamAgent",
    tag = super::TAG,
    params(("agent_id" = AgentId, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "The agent", body = TeamAgentResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to the registry", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such agent in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    TeamAgentPath { agent_id }: TeamAgentPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<TeamAgentResponse>, ApiError> {
    let agent = state
        .usecase
        .get_crew_agent(caller.subject(), agent_id)
        .await?;
    Ok(Json(agent.into()))
}
