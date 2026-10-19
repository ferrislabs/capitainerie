use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::conversation::service::UpdateConversation;
use capitainerie_handlers::{ApiError, Caller};

use crate::{
    dto::{ConversationSummaryResponse, UpdateConversationRequest},
    get_conversation::ConversationPath,
    state::ConversationsState,
};

#[utoipa::path(
    patch,
    path = "/api/v1/conversations/{id}",
    operation_id = "updateConversation",
    tag = super::TAG,
    params(("id" = capitainerie_common::ConversationId, Path, description = "Conversation identifier")),
    request_body = UpdateConversationRequest,
    responses(
        (status = 200, description = "The conversation, renamed and/or archived", body = ConversationSummaryResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents may not change conversations", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such conversation for the caller", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Empty or oversized title", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<R, H, X>(
    ConversationPath { id }: ConversationPath,
    State(state): State<ConversationsState<R, H, X>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateConversationRequest>,
) -> Result<Json<ConversationSummaryResponse>, ApiError> {
    let conversation = state
        .usecase
        .update_conversation(
            caller.subject(),
            id,
            UpdateConversation {
                title: request.title,
                archived: request.archived,
            },
        )
        .await?;
    Ok(Json(ConversationSummaryResponse::from(&conversation)))
}
