use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, CoreError};
use capitainerie_core::domain::execution::{RepositoryIssue, ports::CodeHost};
use capitainerie_handlers::Caller;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/integrations/github/issues")]
pub struct IssuesPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IssuesQuery {
    /// `owner/name`, one of the repositories of the installation.
    pub repository: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IssueResponse {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub labels: Vec<String>,
    /// The card this issue already became, if any.
    pub card_id: Option<CardId>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IssueListResponse {
    pub issues: Vec<IssueResponse>,
}

#[utoipa::path(
    get,
    path = "/api/v1/integrations/github/issues",
    operation_id = "listGithubIssues",
    tag = super::TAG,
    params(IssuesQuery),
    responses(
        (status = 200, description = "The open issues of the repository, pull requests excluded", body = IssueListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "repository_not_allowed: the installation does not reach that repository", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "github_not_connected: the organization has no installation", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Not an owner/name repository, or the GitHub App lacks the issues permission", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: IssuesPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<IssuesQuery>,
) -> Result<Json<IssueListResponse>, IntegrationError> {
    let repository = query.repository.parse()?;
    let subject = caller.subject();
    let installation = state
        .usecase
        .github_installation(subject.clone())
        .await?
        .ok_or_else(|| CoreError::Conflict("github_not_connected".into()))?;
    let reachable = state
        .github
        .apps
        .repositories(subject.organization_id, installation.installation_id)
        .await?;
    if !reachable.contains(&repository) {
        return Err(CoreError::Forbidden {
            reason: Some("repository_not_allowed".into()),
        }
        .into());
    }
    let issues = state
        .github
        .apps
        .open_issues(
            subject.organization_id,
            installation.installation_id,
            &repository,
        )
        .await?;
    let numbers: Vec<u64> = issues.iter().map(|issue| issue.number).collect();
    let imported = state
        .usecase
        .imported_issues(subject, repository, numbers)
        .await?;
    Ok(Json(IssueListResponse {
        issues: issues
            .into_iter()
            .map(|issue| response(issue, &imported))
            .collect(),
    }))
}

fn response(
    issue: RepositoryIssue,
    imported: &[capitainerie_core::domain::worksite::WorksiteIssue],
) -> IssueResponse {
    IssueResponse {
        card_id: imported
            .iter()
            .find(|known| known.number == issue.number)
            .map(|known| known.card_id),
        number: issue.number,
        title: issue.title,
        html_url: issue.html_url,
        labels: issue.labels,
    }
}
