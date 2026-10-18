use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::provider::ports::ModelCatalog;
use capitainerie_handlers::{ApiError, Caller};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    dto::{ModelResponse, TestProviderPath},
    state::ProvidersState,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct TestProviderResponse {
    pub ok: bool,
    /// What the provider answered when it could not be reached.
    pub error: Option<String>,
    pub models: Vec<ModelResponse>,
}

#[utoipa::path(
    post,
    path = "/api/v1/providers/{kind}/test",
    operation_id = "testProvider",
    tag = super::TAG,
    params(("kind" = crate::dto::ProviderKindDto, Path, description = "Provider")),
    responses(
        (status = 200, description = "The outcome of listing the provider's models with the stored key", body = TestProviderResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "The provider is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: ModelCatalog + 'static>(
    TestProviderPath { kind }: TestProviderPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<TestProviderResponse>, ApiError> {
    let kind = kind.into();
    let access = state
        .usecase
        .manageable_provider(caller.subject(), kind)
        .await?;
    let outcome = state.catalog.list_models(&access).await;
    Ok(Json(match outcome {
        Ok(models) => TestProviderResponse {
            ok: true,
            error: None,
            models: models.into_iter().map(ModelResponse::from).collect(),
        },
        Err(error) => TestProviderResponse {
            ok: false,
            error: Some(error.to_string()),
            models: Vec::new(),
        },
    }))
}
