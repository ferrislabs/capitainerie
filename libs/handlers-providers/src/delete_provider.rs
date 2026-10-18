use axum::{Extension, extract::State, http::StatusCode};
use capitainerie_handlers::{ApiError, Caller};

use crate::{dto::ProviderPath, state::ProvidersState};

#[utoipa::path(
    delete,
    path = "/api/v1/providers/{kind}",
    operation_id = "deleteProvider",
    tag = super::TAG,
    params(("kind" = crate::dto::ProviderKindDto, Path, description = "Provider")),
    responses(
        (status = 204, description = "The provider and its key are gone"),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "The provider is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: Send + Sync + 'static>(
    ProviderPath { kind }: ProviderPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<StatusCode, ApiError> {
    state
        .usecase
        .delete_provider(caller.subject(), kind.into())
        .await?;
    state.cache.invalidate(caller.organization.id);
    Ok(StatusCode::NO_CONTENT)
}
