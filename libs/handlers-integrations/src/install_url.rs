use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::CoreError;
use capitainerie_core::domain::authorization::{Action, ResourceKind};
use capitainerie_handlers::Caller;
use chrono::Utc;
use serde::Deserialize;

use crate::{InstallUrlResponse, IntegrationError, signer::StatePurpose, state::IntegrationsState};

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
        (status = 409, description = "github_app_missing: the organization has not created its App yet", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "secrets_key_not_configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: InstallPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<InstallUrlResponse>, IntegrationError> {
    let subject = caller.subject();
    state
        .usecase
        .authorize(
            subject.clone(),
            Action::GithubManage,
            ResourceKind::GithubInstallation,
            None,
        )
        .await?;
    let signer = state.github.signer()?;
    let app = state
        .usecase
        .github_app(subject.clone())
        .await?
        .ok_or_else(|| CoreError::Conflict("github_app_missing".into()))?;
    let token = signer.sign(
        StatePurpose::Install,
        subject.organization_id,
        &subject.id,
        Utc::now(),
    );
    let web_base = state.github.apps.bases().web_base.trim_end_matches('/');
    Ok(Json(InstallUrlResponse {
        url: format!(
            "{web_base}/apps/{}/installations/new?state={token}",
            app.slug
        ),
    }))
}
