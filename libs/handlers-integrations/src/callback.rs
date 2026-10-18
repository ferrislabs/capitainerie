use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::authorization::{Action, ResourceKind};
use capitainerie_handlers::Caller;
use chrono::Utc;
use serde::Deserialize;

use crate::{
    ConnectGithubRequest, GithubIntegrationResponse, IntegrationError, state::IntegrationsState,
};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github/callback")]
pub struct CallbackPath;

#[utoipa::path(
    post,
    path = "/api/v1/integrations/github/callback",
    operation_id = "connectGithub",
    tag = super::TAG,
    request_body = ConnectGithubRequest,
    responses(
        (status = 200, description = "Connected", body = GithubIntegrationResponse),
        (status = 400, description = "The state is forged, expired, or was issued to someone else", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin, or cannot access the installation (installation_not_accessible)", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an admin of the account the App is installed on (installation_not_owned)", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The installation is already connected to an organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "The GitHub App is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: CallbackPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<ConnectGithubRequest>,
) -> Result<Json<GithubIntegrationResponse>, IntegrationError> {
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
    let (client, _, signer) = state.github.installer()?;
    signer.verify(
        &request.state,
        subject.organization_id,
        &subject.id,
        Utc::now(),
    )?;
    let user_token = client.exchange_user_code(&request.code).await?;
    if !client
        .user_can_access_installation(&user_token, request.installation_id)
        .await?
    {
        return Err(IntegrationError::InstallationNotAccessible);
    }
    let account = client.installation_account(request.installation_id).await?;
    if !client.user_owns_installation(&user_token, &account).await? {
        return Err(IntegrationError::InstallationNotOwned);
    }
    drop(user_token);
    let installation = state
        .usecase
        .connect_github(subject, request.installation_id, account.login)
        .await?;
    Ok(Json(GithubIntegrationResponse::new(
        Some(installation),
        true,
    )))
}
