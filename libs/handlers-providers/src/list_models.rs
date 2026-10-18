use std::time::Instant;

use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::provider::ports::ModelCatalog;
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;

use crate::{dto::ModelCatalogResponse, state::ProvidersState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/models")]
pub struct ModelsPath;

#[utoipa::path(
    get,
    description = "Cached for ten minutes per organization, until its providers change.",
    path = "/api/v1/models",
    operation_id = "listModels",
    tag = super::TAG,
    responses(
        (status = 200, description = "What the configured providers offer; an unreachable provider is reported in `errors`", body = ModelCatalogResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: ModelCatalog + 'static>(
    _: ModelsPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ModelCatalogResponse>, ApiError> {
    let organization_id = caller.organization.id;
    let now = Instant::now();
    let listing = match state.cache.get(organization_id, now) {
        Some(cached) => {
            state.usecase.list_providers(caller.subject()).await?;
            cached
        }
        None => {
            let listing = state
                .usecase
                .list_models(caller.subject(), state.catalog.as_ref())
                .await?;
            state.cache.put(organization_id, listing.clone(), now);
            listing
        }
    };
    Ok(Json(listing.into()))
}
