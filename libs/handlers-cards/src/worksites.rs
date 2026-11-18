use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, CoreError, ProjectId, RunId};
use capitainerie_core::domain::{
    card::DESCRIPTION_MAX_CHARS,
    execution::ports::CodeHost,
    worksite::{MAX_WORKSITES_PER_LAUNCH, WorksiteDraft, WorksiteSummary},
};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{execute_card::CodeHostHandle, response::CardResponse};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/projects/{project_id}/worksites")]
pub struct WorksitesPath {
    pub project_id: ProjectId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LaunchWorksitesRequest {
    /// `owner/name`, one of the repositories of the organization's installation.
    pub repository: String,
    /// Free prompts: each becomes a card titled by its first line.
    #[serde(default)]
    pub prompts: Vec<String>,
    /// Numbers of open issues of the repository: each becomes a card.
    #[serde(default)]
    pub issues: Vec<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LaunchedWorksiteResponse {
    pub run_id: RunId,
    pub card: CardResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LaunchWorksitesResponse {
    pub worksites: Vec<LaunchedWorksiteResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorksiteResponse {
    pub card_id: CardId,
    pub title: String,
    pub column: String,
    pub issue_number: Option<u64>,
    pub issue_url: Option<String>,
    pub repository: Option<String>,
    pub run_id: Option<RunId>,
    /// `queued`, `running`, `succeeded`, `failed` or `cancelled`.
    pub run_status: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorksiteListResponse {
    pub items: Vec<WorksiteResponse>,
}

impl From<WorksiteSummary> for WorksiteResponse {
    fn from(summary: WorksiteSummary) -> Self {
        Self {
            card_id: summary.card_id,
            title: summary.title,
            column: summary.column,
            issue_number: summary.issue_number,
            issue_url: summary.issue_url,
            repository: summary.repository.map(|repository| repository.to_string()),
            run_id: summary.run_id,
            run_status: summary.run_status,
            created_at: summary.created_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}/worksites",
    operation_id = "listWorksites",
    tag = super::TAG,
    params(("project_id" = ProjectId, Path, description = "Project identifier")),
    responses(
        (status = 200, description = "The cards of the project that left Proposed, newest first, with their latest execution", body = WorksiteListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such project in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_handler(
    WorksitesPath { project_id }: WorksitesPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<WorksiteListResponse>, ApiError> {
    let items = state
        .usecase
        .list_worksites(caller.subject(), project_id)
        .await?;
    Ok(Json(WorksiteListResponse {
        items: items.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/projects/{project_id}/worksites",
    operation_id = "launchWorksites",
    tag = super::TAG,
    params(("project_id" = ProjectId, Path, description = "Project identifier")),
    request_body = LaunchWorksitesRequest,
    responses(
        (status = 202, description = "Every worksite is a card of the project, In progress, with an Execute run queued", body = LaunchWorksitesResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused, or repository_not_allowed: the installation does not reach that repository", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such project in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "github_not_connected, or issue_already_imported: an issue already has a card", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to open, too many worksites, an empty or oversized prompt, a closed issue, or the GitHub App lacks the issues permission", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn launch_handler(
    WorksitesPath { project_id }: WorksitesPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(CodeHostHandle(github)): Extension<CodeHostHandle>,
    Json(request): Json<LaunchWorksitesRequest>,
) -> Result<(StatusCode, Json<LaunchWorksitesResponse>), ApiError> {
    let repository = request.repository.parse()?;
    let mut issues = request.issues;
    issues.sort_unstable();
    issues.dedup();
    let total = request.prompts.len() + issues.len();
    if total == 0 {
        return Err(CoreError::Invalid("no worksite to open".into()).into());
    }
    if total > MAX_WORKSITES_PER_LAUNCH {
        return Err(CoreError::Invalid(format!(
            "at most {MAX_WORKSITES_PER_LAUNCH} worksites at once"
        ))
        .into());
    }
    let mut drafts = Vec::with_capacity(total);
    for prompt in &request.prompts {
        if prompt.trim().is_empty() {
            return Err(CoreError::Invalid("a prompt is empty".into()).into());
        }
        if prompt.chars().count() > DESCRIPTION_MAX_CHARS {
            return Err(CoreError::Invalid(format!(
                "a prompt exceeds {DESCRIPTION_MAX_CHARS} characters"
            ))
            .into());
        }
        drafts.push(WorksiteDraft::from_prompt(prompt));
    }

    let subject = caller.subject();
    let organization_id = subject.organization_id;
    let not_connected = || CoreError::Conflict("github_not_connected".into());
    if github.client(organization_id).await?.is_none() {
        return Err(not_connected().into());
    }
    let installation = state
        .usecase
        .github_installation(subject.clone())
        .await?
        .ok_or_else(not_connected)?;
    let reachable = github
        .repositories(organization_id, installation.installation_id)
        .await?;
    if !reachable.contains(&repository) {
        return Err(ApiError::Forbidden(Some("repository_not_allowed".into())));
    }
    if !issues.is_empty() {
        let open = github
            .open_issues(organization_id, installation.installation_id, &repository)
            .await?;
        for number in &issues {
            let issue = open
                .iter()
                .find(|issue| issue.number == *number)
                .ok_or_else(|| CoreError::Invalid(format!("issue #{number} is not open")))?;
            drafts.push(WorksiteDraft::from_issue(issue));
        }
    }
    let base_branch = github
        .default_branch(organization_id, installation.installation_id, &repository)
        .await?;
    let started = state
        .usecase
        .launch_worksites(subject, project_id, repository, base_branch, drafts)
        .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(LaunchWorksitesResponse {
            worksites: started
                .into_iter()
                .map(|started| LaunchedWorksiteResponse {
                    run_id: started.run_id,
                    card: started.card.into(),
                })
                .collect(),
        }),
    ))
}
