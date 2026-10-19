use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::ConversationId;
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;

use crate::{
    dto::{ConversationResponse, ConversationSummaryResponse},
    state::ConversationsState,
};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/conversations/{id}")]
pub struct ConversationPath {
    pub id: ConversationId,
}

#[utoipa::path(
    get,
    path = "/api/v1/conversations/{id}",
    operation_id = "getConversation",
    tag = super::TAG,
    params(("id" = ConversationId, Path, description = "Conversation identifier")),
    responses(
        (status = 200, description = "The conversation and its messages, oldest first", body = ConversationResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents may not read conversations", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such conversation for the caller", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<R, H, X>(
    ConversationPath { id }: ConversationPath,
    State(state): State<ConversationsState<R, H, X>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ConversationResponse>, ApiError> {
    let (conversation, messages) = state.usecase.get_conversation(caller.subject(), id).await?;
    Ok(Json(ConversationResponse {
        conversation: ConversationSummaryResponse::from(&conversation),
        messages: messages.into_iter().map(Into::into).collect(),
    }))
}
