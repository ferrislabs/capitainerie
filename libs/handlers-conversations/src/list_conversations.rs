use axum::{Extension, Json, extract::State};
use capitainerie_handlers::{ApiError, Caller};

use crate::{
    create_conversation::ConversationsPath,
    dto::{ConversationListResponse, ConversationSummaryResponse},
    state::ConversationsState,
};

#[utoipa::path(
    get,
    path = "/api/v1/conversations",
    operation_id = "listConversations",
    tag = super::TAG,
    responses(
        (status = 200, description = "The caller's conversations that are not archived, the most recent first", body = ConversationListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents may not read conversations", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<R, H, X>(
    _: ConversationsPath,
    State(state): State<ConversationsState<R, H, X>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ConversationListResponse>, ApiError> {
    let conversations = state.usecase.list_conversations(caller.subject()).await?;
    Ok(Json(ConversationListResponse {
        conversations: conversations
            .iter()
            .map(ConversationSummaryResponse::from)
            .collect(),
    }))
}
