use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_common::AgentId;
use capitainerie_core::domain::crew::service::CreateAgent;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{list_agents::TeamAgentsPath, response::TeamAgentResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTeamAgentRequest {
    /// Kebab-case, unique in the organization.
    pub slug: String,
    pub name: String,
    pub role: Option<String>,
    pub description: Option<String>,
    pub model: Option<String>,
    /// 0 to 360.
    pub hue: Option<i16>,
    pub reports_to: Option<AgentId>,
    pub instructions: Option<String>,
    pub connector_grants: Option<Vec<String>>,
    /// GitHub repositories, `owner/name`, the agent may run the executor on; at most 50.
    pub repository_grants: Option<Vec<String>>,
}

#[utoipa::path(
    post,
    path = "/api/v1/team/agents",
    operation_id = "createTeamAgent",
    tag = super::TAG,
    request_body = CreateTeamAgentRequest,
    responses(
        (status = 201, description = "The new agent, idle", body = TeamAgentResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage the registry", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "An agent of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An invalid field, or a manager that is not an agent of the organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: TeamAgentsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateTeamAgentRequest>,
) -> Result<(StatusCode, Json<TeamAgentResponse>), ApiError> {
    let agent = state
        .usecase
        .create_crew_agent(
            caller.subject(),
            CreateAgent {
                slug: request.slug,
                name: request.name,
                role: request.role.unwrap_or_default(),
                description: request.description.unwrap_or_default(),
                model: request.model.unwrap_or_default(),
                hue: request.hue.unwrap_or_default(),
                reports_to: request.reports_to,
                instructions: request.instructions.unwrap_or_default(),
                connector_grants: request.connector_grants.unwrap_or_default(),
                repository_grants: request.repository_grants.unwrap_or_default(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(agent.into())))
}
