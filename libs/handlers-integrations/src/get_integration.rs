use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::Caller;
use serde::Deserialize;

use crate::{GithubIntegrationResponse, IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github")]
pub struct GithubPath;

#[utoipa::path(
    get,
    path = "/api/v1/integrations/github",
    operation_id = "getGithubIntegration",
    tag = super::TAG,
    responses(
        (status = 200, description = "The organization's GitHub App and where it is installed", body = GithubIntegrationResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: GithubPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<GithubIntegrationResponse>, IntegrationError> {
    let subject = caller.subject();
    let app = state.usecase.github_app(subject.clone()).await?;
    let installation = state.usecase.github_installation(subject).await?;
    Ok(Json(GithubIntegrationResponse::new(app, installation)))
}
