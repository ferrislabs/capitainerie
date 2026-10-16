use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Serialize;
use utoipa::ToSchema;

use crate::response::CardResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/cards")]
pub struct CardsPath;

#[derive(Debug, Serialize, ToSchema)]
pub struct CardListResponse {
    pub items: Vec<CardResponse>,
}

#[utoipa::path(
    get,
    path = "/api/v1/cards",
    operation_id = "listCards",
    tag = super::TAG,
    responses(
        (status = 200, description = "The organization's board, most recently changed first", body = CardListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: CardsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<CardListResponse>, ApiError> {
    let cards = state.usecase.list_cards(caller.subject()).await?;
    Ok(Json(CardListResponse {
        items: cards.into_iter().map(CardResponse::from).collect(),
    }))
}
