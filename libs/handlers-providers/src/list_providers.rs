use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::provider::ProviderKind;
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;

use crate::{
    dto::{ProviderListResponse, ProviderResponse},
    state::ProvidersState,
};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/providers")]
pub struct ProvidersPath;

#[utoipa::path(
    get,
    path = "/api/v1/providers",
    operation_id = "listProviders",
    tag = super::TAG,
    responses(
        (status = 200, description = "The providers configured for the organization, without their keys", body = ProviderListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: Send + Sync + 'static>(
    _: ProvidersPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ProviderListResponse>, ApiError> {
    let mut providers = state.usecase.list_providers(caller.subject()).await?;
    providers.sort_by_key(|summary| ProviderKind::ALL.iter().position(|k| *k == summary.kind));
    Ok(Json(ProviderListResponse {
        providers: providers.into_iter().map(ProviderResponse::from).collect(),
    }))
}
