use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::CardId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::CardDetailResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}")]
pub struct GetCardPath {
    pub card_id: CardId,
}

#[utoipa::path(
    get,
    path = "/api/v1/cards/{card_id}",
    operation_id = "getCard",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    responses(
        (status = 200, description = "The card with its comments and runs", body = CardDetailResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    GetCardPath { card_id }: GetCardPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<CardDetailResponse>, ApiError> {
    let detail = state.usecase.card_detail(caller.subject(), card_id).await?;
    Ok(Json(detail.into()))
}
