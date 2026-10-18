use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::CoreError;
use capitainerie_core::domain::execution::ports::CodeHost;
use capitainerie_handlers::Caller;
use serde::Deserialize;

use crate::{IntegrationError, RepositoryListResponse, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github/repositories")]
pub struct RepositoriesPath;

#[utoipa::path(
    get,
    path = "/api/v1/integrations/github/repositories",
    operation_id = "listGithubRepositories",
    tag = super::TAG,
    responses(
        (status = 200, description = "The repositories the installation can see", body = RepositoryListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "github_not_connected: the organization has no installation", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "The GitHub App is not configured", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: RepositoriesPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<RepositoryListResponse>, IntegrationError> {
    let client = state.github.client()?;
    let installation = state
        .usecase
        .github_installation(caller.subject())
        .await?
        .ok_or_else(|| CoreError::Conflict("github_not_connected".into()))?;
    let repositories = client.repositories(installation.installation_id).await?;
    Ok(Json(RepositoryListResponse {
        repositories: repositories.iter().map(Into::into).collect(),
    }))
}
