use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::Caller;
use serde::Deserialize;

use crate::{GithubIntegrationResponse, IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github")]
pub struct DisconnectPath;

#[utoipa::path(
    delete,
    path = "/api/v1/integrations/github",
    operation_id = "disconnectGithub",
    tag = super::TAG,
    responses(
        (status = 200, description = "Disconnected; running executions keep the grants they already hold", body = GithubIntegrationResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: DisconnectPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<GithubIntegrationResponse>, IntegrationError> {
    let subject = caller.subject();
    state.usecase.disconnect_github(subject.clone()).await?;
    let app = state.usecase.github_app(subject).await?;
    Ok(Json(GithubIntegrationResponse::new(app, None)))
}
