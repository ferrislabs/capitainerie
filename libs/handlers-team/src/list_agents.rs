use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::TeamAgentListResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/team/agents")]
pub struct TeamAgentsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListTeamAgentsQuery {
    pub archived: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/v1/team/agents",
    operation_id = "listTeamAgents",
    tag = super::TAG,
    params(ListTeamAgentsQuery),
    responses(
        (status = 200, description = "The team's agents by name; without archived, the active ones only", body = TeamAgentListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to the registry", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: TeamAgentsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListTeamAgentsQuery>,
) -> Result<Json<TeamAgentListResponse>, ApiError> {
    let agents = state
        .usecase
        .list_crew(caller.subject(), query.archived)
        .await?;
    Ok(Json(TeamAgentListResponse {
        items: agents.into_iter().map(Into::into).collect(),
    }))
}
