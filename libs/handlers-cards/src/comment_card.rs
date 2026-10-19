use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::CardId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::response::CommentResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}/comments")]
pub struct CommentCardPath {
    pub card_id: CardId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CommentCardRequest {
    /// Not empty once trimmed, 10 000 characters at most.
    pub body: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/cards/{card_id}/comments",
    operation_id = "commentCard",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    request_body = CommentCardRequest,
    responses(
        (status = 201, description = "The comment, signed by the caller", body = CommentResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "An agent caller: agents comment through the MCP server", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The comment is empty or longer than 10 000 characters", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    CommentCardPath { card_id }: CommentCardPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CommentCardRequest>,
) -> Result<(StatusCode, Json<CommentResponse>), ApiError> {
    let subject = caller.subject();
    if subject.actor.is_some() {
        return Err(ApiError::Forbidden(Some(
            "agents comment through the MCP server".into(),
        )));
    }
    let comment = state
        .usecase
        .comment_card(subject, card_id, request.body, None)
        .await?;
    Ok((StatusCode::CREATED, Json(comment.into())))
}
