use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::provider::ProviderKind;
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;

use crate::{dto::UserProviderResponse, state::ProvidersState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/me/providers")]
pub struct UserProvidersPath;

#[utoipa::path(
    get,
    path = "/api/v1/me/providers",
    operation_id = "listMyProviders",
    tag = super::TAG,
    responses(
        (status = 200, description = "The providers the caller connected with their own key, without the keys", body = Vec<UserProviderResponse>),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: Send + Sync + 'static>(
    _: UserProvidersPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<Vec<UserProviderResponse>>, ApiError> {
    let mut providers = state.usecase.list_user_providers(caller.subject()).await?;
    providers.sort_by_key(|summary| ProviderKind::ALL.iter().position(|k| *k == summary.kind));
    Ok(Json(
        providers
            .into_iter()
            .map(UserProviderResponse::from)
            .collect(),
    ))
}
