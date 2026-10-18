use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::{agent::SecretToken, provider::service::PutProvider};
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    dto::{ProviderPath, ProviderResponse},
    state::ProvidersState,
};

/// Neither `Debug` nor `Serialize`: the key must not end up in a log line.
#[derive(Deserialize, ToSchema)]
pub struct PutProviderRequest {
    /// Empty or absent only for an `openai_compatible` endpoint without a key.
    #[serde(default)]
    pub api_key: String,
    /// Required for `openai_compatible`, ignored otherwise.
    pub base_url: Option<String>,
}

#[utoipa::path(
    put,
    path = "/api/v1/providers/{kind}",
    operation_id = "putProvider",
    tag = super::TAG,
    params(("kind" = crate::dto::ProviderKindDto, Path, description = "Provider")),
    request_body = PutProviderRequest,
    responses(
        (status = 200, description = "The provider as stored; the key is not returned", body = ProviderResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Invalid key or base URL", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: Send + Sync + 'static>(
    ProviderPath { kind }: ProviderPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<PutProviderRequest>,
) -> Result<Json<ProviderResponse>, ApiError> {
    let summary = state
        .usecase
        .put_provider(PutProvider {
            subject: caller.subject(),
            kind: kind.into(),
            secret: SecretToken::new(request.api_key),
            base_url: request.base_url,
        })
        .await?;
    state.cache.invalidate(caller.organization.id);
    Ok(Json(summary.into()))
}
