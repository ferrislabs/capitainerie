use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::provider::ports::ModelCatalog;
use capitainerie_handlers::{ApiError, Caller};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{dto::TestUserProviderPath, state::ProvidersState};

#[derive(Debug, Serialize, ToSchema)]
pub struct TestUserProviderResponse {
    pub ok: bool,
    /// What the provider answered when it could not be reached, without the key.
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/me/providers/{kind}/test",
    operation_id = "testMyProvider",
    tag = super::TAG,
    params(("kind" = crate::dto::ProviderKindDto, Path, description = "Provider")),
    responses(
        (status = 200, description = "The outcome of a live call with the caller's stored key", body = TestUserProviderResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "The caller has no such connection", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: ModelCatalog + 'static>(
    TestUserProviderPath { kind }: TestUserProviderPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<TestUserProviderResponse>, ApiError> {
    let outcome = state
        .usecase
        .test_user_provider(caller.subject(), kind.into(), state.catalog.as_ref())
        .await?;
    Ok(Json(match outcome {
        Ok(()) => TestUserProviderResponse {
            ok: true,
            error: None,
        },
        Err(message) => TestUserProviderResponse {
            ok: false,
            error: Some(message),
        },
    }))
}
