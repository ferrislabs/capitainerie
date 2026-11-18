use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_common::{AgentId, CardId, ClientId, ProjectId};
use capitainerie_core::domain::card::{attributes::Priority, service::CreateTask};
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    list_cards::CardsPath,
    response::{CardResponse, PriorityDto},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCardRequest {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<ProjectId>,
    pub client_id: Option<ClientId>,
    /// Normal when absent.
    pub priority: Option<PriorityDto>,
    /// An active agent of the team registry.
    pub assignee_agent_id: Option<AgentId>,
    /// Makes the card a subtask; the nesting stops at three levels.
    pub parent_id: Option<CardId>,
}

#[utoipa::path(
    post,
    path = "/api/v1/cards",
    operation_id = "createCard",
    tag = super::TAG,
    request_body = CreateCardRequest,
    responses(
        (status = 201, description = "The new card, in the todo column", body = CardResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Only a human may create a card by hand", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An invalid title or description, an unknown or archived client, project or agent, a parent that is not in the organization, or a nesting over three levels", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: CardsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), ApiError> {
    let card = state
        .usecase
        .create_task(
            caller.subject(),
            CreateTask {
                title: request.title,
                description: request.description.unwrap_or_default(),
                priority: request.priority.map_or(Priority::Normal, Into::into),
                client: request.client_id,
                project: request.project_id,
                assignee_agent: request.assignee_agent_id,
                parent: request.parent_id,
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(card.into())))
}
