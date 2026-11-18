use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::{agent::SecretToken, user_provider::service::PutUserProvider};
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    dto::{UserProviderPath, UserProviderResponse},
    state::ProvidersState,
};

/// Neither `Debug` nor `Serialize`: the key must not end up in a log line.
#[derive(Deserialize, ToSchema)]
pub struct PutUserProviderRequest {
    /// Absent keeps the stored key untouched, so the model, base URL or default
    /// flag can change without re-sending the secret; it is then refused when
    /// no connection of that kind exists yet, except for `openai_compatible`,
    /// which may have no key. Empty means no key, accepted only for
    /// `openai_compatible`.
    pub api_key: Option<String>,
    /// Required for `openai_compatible` (public https host), ignored otherwise.
    pub base_url: Option<String>,
    /// The model the caller's agent runs use when the agent names none.
    pub default_model: String,
    /// Makes this provider the caller's default. The first one connected is
    /// the default whatever is sent.
    pub is_default: Option<bool>,
}

#[utoipa::path(
    put,
    path = "/api/v1/me/providers/{kind}",
    operation_id = "putMyProvider",
    tag = super::TAG,
    params(("kind" = crate::dto::ProviderKindDto, Path, description = "Provider")),
    request_body = PutUserProviderRequest,
    responses(
        (status = 200, description = "The connection as stored; the key is not returned", body = UserProviderResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Invalid or missing key, invalid model or base URL", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: Send + Sync + 'static>(
    UserProviderPath { kind }: UserProviderPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<PutUserProviderRequest>,
) -> Result<Json<UserProviderResponse>, ApiError> {
    let summary = state
        .usecase
        .put_user_provider(PutUserProvider {
            subject: caller.subject(),
            kind: kind.into(),
            secret: request.api_key.map(SecretToken::new),
            base_url: request.base_url,
            default_model: request.default_model,
            is_default: request.is_default,
        })
        .await?;
    Ok(Json(summary.into()))
}
