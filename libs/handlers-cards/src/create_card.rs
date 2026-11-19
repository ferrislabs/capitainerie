use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_common::{AgentId, CardId, ClientId, ProjectId};
use capitainerie_core::domain::{
    card::{attributes::Priority, service::CreateTask},
    execution::RepositoryRef,
};
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auto_run::{AgentRunOutcome, Handoff, hand_off},
    execute_card::CodeHostHandle,
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
    /// `owner/name`, one of the agent's repositories: where it works when it
    /// has several. Ignored without an agent.
    #[serde(default)]
    pub repository: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedCardResponse {
    #[serde(flatten)]
    pub card: CardResponse,
    /// With an agent: whether it started on the card.
    pub agent_run: Option<AgentRunOutcome>,
}

#[utoipa::path(
    post,
    path = "/api/v1/cards",
    operation_id = "createCard",
    tag = super::TAG,
    request_body = CreateCardRequest,
    responses(
        (status = 201, description = "The new card, in the todo column; with an agent, the agent starts on it, writes off (each write asks for an approval), and `agent_run` says whether it did", body = CreatedCardResponse),
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
    Extension(CodeHostHandle(github)): Extension<CodeHostHandle>,
    Json(request): Json<CreateCardRequest>,
) -> Result<(StatusCode, Json<CreatedCardResponse>), ApiError> {
    let repository = request
        .repository
        .as_deref()
        .map(str::parse::<RepositoryRef>)
        .transpose()?;
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
    let agent_run = match card.attributes.assignee_agent_id {
        Some(_) => {
            hand_off(
                &state,
                &github,
                caller.subject(),
                card.id,
                Handoff::Created { repository },
            )
            .await
        }
        None => None,
    };
    Ok((
        StatusCode::CREATED,
        Json(CreatedCardResponse {
            card: card.into(),
            agent_run,
        }),
    ))
}
