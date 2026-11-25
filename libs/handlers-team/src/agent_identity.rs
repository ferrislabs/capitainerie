use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::AgentId;
use capitainerie_core::domain::agent_identity::{AgentClientId, IdentityStatus};
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/team/agents/{agent_id}/identity")]
pub struct TeamAgentIdentityPath {
    pub agent_id: AgentId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TeamAgentIdentityStatusDto {
    /// The client is not in FerrisKey yet; the worker retries.
    Pending,
    Active,
    /// The agent is archived and its client cannot obtain a token.
    Disabled,
}

/// The agent's own client in FerrisKey. Its secret never leaves Capitainerie.
#[derive(Debug, Serialize, ToSchema)]
pub struct TeamAgentIdentityResponse {
    pub client_id: String,
    pub status: TeamAgentIdentityStatusDto,
    /// Why the last attempt to create the client stopped, while pending.
    pub failure: Option<String>,
    /// False while FerrisKey has yet to follow an archive or an unarchive.
    pub synced: bool,
}

#[utoipa::path(
    get,
    path = "/api/v1/team/agents/{agent_id}/identity",
    operation_id = "getTeamAgentIdentity",
    tag = super::TAG,
    params(("agent_id" = AgentId, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "The agent's FerrisKey client and its status", body = TeamAgentIdentityResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to the registry", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such agent in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    TeamAgentIdentityPath { agent_id }: TeamAgentIdentityPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<TeamAgentIdentityResponse>, ApiError> {
    let summary = state
        .usecase
        .crew_agent_identity(caller.subject(), agent_id)
        .await?;
    let response = match summary {
        Some(summary) => TeamAgentIdentityResponse {
            client_id: summary.client_id.as_str().to_owned(),
            status: summary.status.into(),
            failure: summary.failure,
            synced: summary.synced,
        },
        None => TeamAgentIdentityResponse {
            client_id: AgentClientId::of(agent_id).as_str().to_owned(),
            status: TeamAgentIdentityStatusDto::Pending,
            failure: None,
            synced: true,
        },
    };
    Ok(Json(response))
}

impl From<IdentityStatus> for TeamAgentIdentityStatusDto {
    fn from(status: IdentityStatus) -> Self {
        match status {
            IdentityStatus::Pending => Self::Pending,
            IdentityStatus::Active => Self::Active,
            IdentityStatus::Disabled => Self::Disabled,
        }
    }
}
