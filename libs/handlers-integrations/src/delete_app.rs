use axum::{Extension, Json, extract::State};
use capitainerie_handlers::Caller;

use crate::{GithubAppResponse, IntegrationError, state::IntegrationsState};

pub use crate::create_app::AppPath as DeleteAppPath;

#[utoipa::path(
    delete,
    path = "/api/v1/integrations/github/app",
    operation_id = "deleteGithubApp",
    tag = super::TAG,
    responses(
        (status = 200, description = "The App and the installation of the organization are forgotten; the App stays on GitHub, to be deleted at its html_url", body = GithubAppResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "The organization has no App", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: DeleteAppPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<GithubAppResponse>, IntegrationError> {
    let removed = state.usecase.delete_github_app(caller.subject()).await?;
    Ok(Json(removed.into()))
}
