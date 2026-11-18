use axum::{Extension, extract::State, http::StatusCode};
use capitainerie_handlers::{ApiError, Caller};

use crate::{dto::UserProviderPath, state::ProvidersState};

#[utoipa::path(
    delete,
    path = "/api/v1/me/providers/{kind}",
    operation_id = "deleteMyProvider",
    tag = super::TAG,
    params(("kind" = crate::dto::ProviderKindDto, Path, description = "Provider")),
    responses(
        (status = 204, description = "The connection and its key are gone; another one becomes the default if it was"),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "The caller has no such connection", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: Send + Sync + 'static>(
    UserProviderPath { kind }: UserProviderPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<StatusCode, ApiError> {
    state
        .usecase
        .delete_user_provider(caller.subject(), kind.into())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
