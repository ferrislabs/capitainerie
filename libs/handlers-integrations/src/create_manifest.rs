use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::CoreError;
use capitainerie_core::domain::authorization::{Action, ResourceKind};
use capitainerie_handlers::Caller;
use chrono::Utc;
use serde::Deserialize;

use crate::{
    CreateManifestRequest, IntegrationError, ManifestResponse, manifest, signer::StatePurpose,
    state::IntegrationsState,
};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github/app/manifest")]
pub struct ManifestPath;

#[utoipa::path(
    post,
    path = "/api/v1/integrations/github/app/manifest",
    operation_id = "createGithubAppManifest",
    tag = super::TAG,
    request_body = CreateManifestRequest,
    responses(
        (status = 200, description = "The manifest the browser POSTs to GitHub to create the organization's App; the state expires in 10 minutes", body = ManifestResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "github_app_exists: the organization already has an App", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Not a GitHub organization login", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "console_url_not_configured or secrets_key_not_configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: ManifestPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateManifestRequest>,
) -> Result<Json<ManifestResponse>, IntegrationError> {
    let subject = caller.subject();
    state
        .usecase
        .authorize(
            subject.clone(),
            Action::GithubManage,
            ResourceKind::GithubApp,
            None,
        )
        .await?;
    let signer = state.github.signer()?;
    let console_url = state.github.console_url()?;
    if state.usecase.github_app(subject.clone()).await?.is_some() {
        return Err(CoreError::Conflict("github_app_exists".into()).into());
    }
    let token = signer.sign(
        StatePurpose::Manifest,
        subject.organization_id,
        &subject.id,
        Utc::now(),
    );
    let action_url =
        manifest::action_url(&state.github.apps.bases().web_base, &request.owner, &token)?;
    let name = manifest::app_name(&caller.organization.name, &manifest::random_suffix()?);
    Ok(Json(ManifestResponse {
        action_url,
        manifest: manifest::manifest(console_url, &name),
        state: token,
    }))
}
