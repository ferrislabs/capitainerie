use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::authorization::{AccessRequest, Action, ResourceKind, require};
use capitainerie_handlers::Caller;
use chrono::Utc;
use serde::Deserialize;

use crate::{InstallUrlResponse, IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github/install")]
pub struct InstallPath;

#[utoipa::path(
    get,
    path = "/api/v1/integrations/github/install",
    operation_id = "getGithubInstallUrl",
    tag = super::TAG,
    responses(
        (status = 200, description = "Where to send the admin to install the GitHub App; the state expires in 10 minutes", body = InstallUrlResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "The GitHub App is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: InstallPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<InstallUrlResponse>, IntegrationError> {
    let subject = caller.subject();
    require(
        &state.authz,
        AccessRequest::new(
            subject.clone(),
            Action::GithubManage,
            ResourceKind::GithubInstallation,
            None,
        ),
    )
    .await?;
    let (_, slug, signer) = state.github.installer()?;
    let token = signer.sign(subject.organization_id, &subject.id, Utc::now());
    Ok(Json(InstallUrlResponse {
        url: format!("https://github.com/apps/{slug}/installations/new?state={token}"),
    }))
}
