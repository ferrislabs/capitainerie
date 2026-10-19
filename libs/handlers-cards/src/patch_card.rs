use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, ClientId, ProjectId};
use capitainerie_core::domain::{card::edit::EditCard, client::links::Change};
use capitainerie_handlers::{ApiError, AppState, Caller, Patch};
use chrono::NaiveDate;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::response::{CardResponse, CriterionDto, EstimateDto, PriorityDto};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}")]
pub struct PatchCardPath {
    pub card_id: CardId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchCardRequest {
    pub expected_version: i32,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub title: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<CriterionDto>>)]
    pub acceptance_criteria: Patch<Vec<CriterionDto>>,
    #[serde(default)]
    #[schema(value_type = Option<PriorityDto>)]
    pub priority: Patch<PriorityDto>,
    #[serde(default)]
    #[schema(value_type = Option<String>, format = Date)]
    pub due_date: Patch<NaiveDate>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub assignee_subject: Patch<String>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<String>>)]
    pub labels: Patch<Vec<String>>,
    #[serde(default)]
    #[schema(value_type = Option<EstimateDto>)]
    pub estimate: Patch<EstimateDto>,
    #[serde(default)]
    #[schema(value_type = Option<ClientId>)]
    pub client_id: Patch<ClientId>,
    #[serde(default)]
    #[schema(value_type = Option<ProjectId>)]
    pub project_id: Patch<ProjectId>,
}

#[utoipa::path(
    patch,
    path = "/api/v1/cards/{card_id}",
    operation_id = "patchCard",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    request_body = PatchCardRequest,
    responses(
        (status = 200, description = "The card as edited; absent fields are unchanged, null clears a field (a priority goes back to normal, a title cannot be cleared)", body = CardResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may never edit a card through this route", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The card changed since it was read", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An invalid value (criteria, labels, title), an assignee who is not a member, an unknown or archived client or project, one of another organization, or a client that is not the project's", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    PatchCardPath { card_id }: PatchCardPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<PatchCardRequest>,
) -> Result<Json<CardResponse>, ApiError> {
    let card = state
        .usecase
        .edit_card(
            caller.subject(),
            card_id,
            EditCard {
                expected_version: request.expected_version,
                title: request.title.change(),
                description: request.description.change(),
                acceptance_criteria: match request.acceptance_criteria.change() {
                    Change::Set(criteria) => {
                        Change::Set(criteria.into_iter().map(Into::into).collect())
                    }
                    Change::Keep => Change::Keep,
                    Change::Clear => Change::Clear,
                },
                priority: map_change(request.priority.change()),
                due_date: request.due_date.change(),
                assignee_subject: request.assignee_subject.change(),
                labels: request.labels.change(),
                estimate: map_change(request.estimate.change()),
                client: request.client_id.change(),
                project: request.project_id.change(),
            },
        )
        .await?;
    Ok(Json(card.into()))
}

fn map_change<T, U: From<T>>(change: Change<T>) -> Change<U> {
    match change {
        Change::Keep => Change::Keep,
        Change::Clear => Change::Clear,
        Change::Set(value) => Change::Set(value.into()),
    }
}
