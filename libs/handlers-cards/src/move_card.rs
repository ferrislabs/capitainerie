use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::CardId;
use capitainerie_core::domain::card::service::MoveCard;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::response::{CardResponse, ColumnDto};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}/move")]
pub struct MoveCardPath {
    pub card_id: CardId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveCardRequest {
    pub to: ColumnDto,
    /// The version the caller last read; a stale one is refused with 409.
    pub expected_version: i32,
}

#[utoipa::path(
    post,
    path = "/api/v1/cards/{card_id}/move",
    operation_id = "moveCard",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    request_body = MoveCardRequest,
    responses(
        (status = 200, description = "The moved card", body = CardResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the move", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The card changed since it was read", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    MoveCardPath { card_id }: MoveCardPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<MoveCardRequest>,
) -> Result<Json<CardResponse>, ApiError> {
    let card = state
        .usecase
        .move_card(
            caller.subject(),
            card_id,
            MoveCard {
                to: request.to.into(),
                expected_version: request.expected_version,
                run_id: None,
            },
        )
        .await?;
    Ok(Json(card.into()))
}
