use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, Caller};

use crate::{
    dto::{CreateConversationRequest, CreatedConversationResponse},
    state::ConversationsState,
};

#[derive(TypedPath)]
#[typed_path("/api/v1/conversations")]
pub struct ConversationsPath;

#[utoipa::path(
    post,
    path = "/api/v1/conversations",
    operation_id = "createConversation",
    tag = super::TAG,
    request_body = CreateConversationRequest,
    responses(
        (status = 201, description = "The conversation exists with the member's first message; open the stream of the first turn with sendConversationMessage", body = CreatedConversationResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents may not open conversations", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Empty or oversized message", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<R, H, X>(
    _: ConversationsPath,
    State(state): State<ConversationsState<R, H, X>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<CreatedConversationResponse>), ApiError> {
    let conversation = state
        .usecase
        .create_conversation(caller.subject(), request.message)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedConversationResponse {
            id: conversation.id,
        }),
    ))
}
