use axum::{Extension, Json, extract::State};
use capitainerie_common::AgentId;
use capitainerie_core::domain::crew::service::UpdateAgent;
use capitainerie_handlers::{ApiError, AppState, Caller, Patch};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{get_agent::TeamAgentPath, response::TeamAgentResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTeamAgentRequest {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub description: Option<String>,
    pub model: Option<String>,
    pub hue: Option<i16>,
    /// null removes the manager.
    #[serde(default)]
    #[schema(value_type = Option<AgentId>)]
    pub reports_to: Patch<AgentId>,
    pub archived: Option<bool>,
    pub instructions: Option<String>,
    /// Replaces the whole list.
    pub connector_grants: Option<Vec<String>>,
}

#[utoipa::path(
    patch,
    path = "/api/v1/team/agents/{agent_id}",
    operation_id = "updateTeamAgent",
    tag = super::TAG,
    params(("agent_id" = AgentId, Path, description = "Agent identifier")),
    request_body = UpdateTeamAgentRequest,
    responses(
        (status = 200, description = "The updated agent; absent fields are unchanged", body = TeamAgentResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage the registry", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such agent in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "An agent of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, an invalid field, a manager that is not an agent of the organization, or a reporting loop", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    TeamAgentPath { agent_id }: TeamAgentPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateTeamAgentRequest>,
) -> Result<Json<TeamAgentResponse>, ApiError> {
    let agent = state
        .usecase
        .update_crew_agent(
            caller.subject(),
            agent_id,
            UpdateAgent {
                slug: request.slug,
                name: request.name,
                role: request.role,
                description: request.description,
                model: request.model,
                hue: request.hue,
                reports_to: request.reports_to.change(),
                archived: request.archived,
                instructions: request.instructions,
                connector_grants: request.connector_grants,
            },
        )
        .await?;
    Ok(Json(agent.into()))
}
