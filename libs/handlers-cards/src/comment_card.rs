use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::CardId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auto_run::{AgentRunOutcome, Handoff, hand_off},
    execute_card::CodeHostHandle,
    response::CommentResponse,
};

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

#[derive(Debug, Serialize, ToSchema)]
pub struct PostedCommentResponse {
    #[serde(flatten)]
    pub comment: CommentResponse,
    /// When the card has an agent: whether the reply started it again.
    pub agent_run: Option<AgentRunOutcome>,
}

#[utoipa::path(
    post,
    path = "/api/v1/cards/{card_id}/comments",
    operation_id = "commentCard",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    request_body = CommentCardRequest,
    responses(
        (status = 201, description = "The comment, signed by the caller; on a card with an agent, the agent starts again and reads the whole thread, and `agent_run` says whether it did", body = PostedCommentResponse),
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
    Extension(CodeHostHandle(github)): Extension<CodeHostHandle>,
    Json(request): Json<CommentCardRequest>,
) -> Result<(StatusCode, Json<PostedCommentResponse>), ApiError> {
    let subject = caller.subject();
    if subject.actor.is_some() {
        return Err(ApiError::Forbidden(Some(
            "agents comment through the MCP server".into(),
        )));
    }
    let comment = state
        .usecase
        .comment_card(subject.clone(), card_id, request.body, None)
        .await?;
    let agent_run = hand_off(&state, &github, subject, card_id, Handoff::Replied).await;
    Ok((
        StatusCode::CREATED,
        Json(PostedCommentResponse {
            comment: comment.into(),
            agent_run,
        }),
    ))
}
