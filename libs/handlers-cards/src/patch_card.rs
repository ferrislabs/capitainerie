use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, ClientId, ProjectId};
use capitainerie_core::domain::card::link::LinkCard;
use capitainerie_handlers::{ApiError, AppState, Caller, Patch};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::response::CardResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}")]
pub struct PatchCardPath {
    pub card_id: CardId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchCardRequest {
    pub expected_version: i32,
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
        (status = 200, description = "The card with its new links", body = CardResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may never link a card", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The card changed since it was read", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Unknown or archived client or project, one of another organization, or a client that is not the project's", body = capitainerie_handlers::errors::ApiErrorResponse),
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
        .link_card(
            caller.subject(),
            card_id,
            LinkCard {
                expected_version: request.expected_version,
                client: request.client_id.change(),
                project: request.project_id.change(),
            },
        )
        .await?;
    Ok(Json(card.into()))
}
