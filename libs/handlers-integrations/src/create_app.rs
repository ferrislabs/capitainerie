use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::CoreError;
use capitainerie_core::domain::authorization::{Action, ResourceKind};
use capitainerie_handlers::Caller;
use chrono::Utc;
use serde::Deserialize;

use crate::{
    CreateGithubAppRequest, GithubAppResponse, IntegrationError, signer::StatePurpose,
    state::IntegrationsState,
};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github/app")]
pub struct AppPath;

#[utoipa::path(
    post,
    path = "/api/v1/integrations/github/app",
    operation_id = "createGithubApp",
    tag = super::TAG,
    request_body = CreateGithubAppRequest,
    responses(
        (status = 201, description = "GitHub created the App; its secrets are stored sealed and never returned", body = GithubAppResponse),
        (status = 400, description = "The state is forged, expired, or was issued to someone else or for something else", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "github_app_exists: the organization already has an App", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "GitHub refused the code (expired or already used)", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "secrets_key_not_configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: AppPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateGithubAppRequest>,
) -> Result<(StatusCode, Json<GithubAppResponse>), IntegrationError> {
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
    state.github.signer()?.verify(
        StatePurpose::Manifest,
        &request.state,
        subject.organization_id,
        &subject.id,
        Utc::now(),
    )?;
    if state.usecase.github_app(subject.clone()).await?.is_some() {
        return Err(CoreError::Conflict("github_app_exists".into()).into());
    }
    let conversion = state.github.apps.convert_manifest(&request.code).await?;
    let credentials = conversion.into_credentials(subject.organization_id, subject.id.clone());
    let summary = state.usecase.save_github_app(subject, credentials).await?;
    Ok((StatusCode::CREATED, Json(summary.into())))
}
