use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_core::{
    RunDelegation,
    domain::{
        agent_task::{AgentRunError, AgentRunRefusal, RepositoryTarget, StartAgentRun},
        authorization::Subject,
        execution::{RepositoryRef, ports::CodeHost},
    },
};
use capitainerie_github::GithubApps;
use capitainerie_handlers::{ApiError, AppState, Caller, errors::ApiErrorResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::execute_card::CodeHostHandle;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}/agent-runs")]
pub struct RunAgentPath {
    pub card_id: CardId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RunAgentRequest {
    pub expected_version: i32,
    /// Without a repository: lets the agent's connectors write. With one:
    /// `false` runs a read-only audit that files a report, `true` lets it push
    /// a branch and open a draft pull request.
    #[serde(default)]
    pub allow_writes: bool,
    /// `owner/name`: runs the agent in the executor on this repository, one
    /// of the agent's `repository_grants`, instead of with its connectors.
    #[serde(default)]
    pub repository: Option<String>,
    /// The branch to check out; the repository's default branch when absent.
    /// Ignored without `repository`.
    #[serde(default)]
    pub base_branch: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunAgentResponse {
    pub run_id: RunId,
}

#[derive(Debug)]
pub enum RunAgentError {
    Api(ApiError),
    Refused(AgentRunRefusal),
}

impl From<AgentRunError> for RunAgentError {
    fn from(error: AgentRunError) -> Self {
        match error {
            AgentRunError::Core(error) => Self::Api(error.into()),
            AgentRunError::Refused(refusal) => Self::Refused(refusal),
        }
    }
}

impl From<CoreError> for RunAgentError {
    fn from(error: CoreError) -> Self {
        Self::Api(error.into())
    }
}

impl IntoResponse for RunAgentError {
    fn into_response(self) -> Response {
        match self {
            Self::Api(error) => error.into_response(),
            Self::Refused(refusal) => {
                let status = match refusal {
                    AgentRunRefusal::NotConnected(_)
                    | AgentRunRefusal::IdentityNotActive
                    | AgentRunRefusal::NoProvider
                    | AgentRunRefusal::GithubNotConnected
                    | AgentRunRefusal::ExecutorDisabled(_) => StatusCode::CONFLICT,
                    _ => StatusCode::UNPROCESSABLE_ENTITY,
                };
                let body = ApiErrorResponse {
                    code: refusal.code(),
                    status: status.as_u16(),
                    message: refusal.to_string(),
                };
                (status, Json(body)).into_response()
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/cards/{card_id}/agent-runs",
    operation_id = "runCardAgent",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    request_body = RunAgentRequest,
    responses(
        (status = 202, description = "A run of the card's assignee agent is queued: with its connectors, or with `repository` an Execute run in the sandboxed executor (audit when writes are not allowed, change otherwise)", body = RunAgentResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused (an agent cannot start an agent run), or repository_not_allowed: the GitHub installation does not reach that repository", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "not_connected (the message names the connector slug), agent_identity_not_active (the agent's FerrisKey client is not ready yet), no_provider (neither the caller nor the organization connected an LLM provider; with `repository`, one of the provider of the harness the agent's model runs on, Anthropic when no harness takes it), github_not_connected, executor_disabled, execution_already_active, or the card changed since it was read", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "card_without_agent, agent_archived, agent_paused, agent_without_connector (without `repository`), repository_not_granted (the agent may not work on that repository), or an invalid repository or base_branch", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    RunAgentPath { card_id }: RunAgentPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(CodeHostHandle(github)): Extension<CodeHostHandle>,
    Json(request): Json<RunAgentRequest>,
) -> Result<(StatusCode, Json<RunAgentResponse>), RunAgentError> {
    let command = StartAgentRun {
        expected_version: request.expected_version,
        allow_writes: request.allow_writes,
    };
    let started = match request.repository {
        None => start_with_connectors(&state, &caller, card_id, command).await?,
        Some(repository) => {
            let target = RepositoryTarget::new(repository.parse()?, request.base_branch)?;
            start_on_repository(&state, &github, caller.subject(), card_id, command, target).await?
        }
    };
    Ok((
        StatusCode::ACCEPTED,
        Json(RunAgentResponse {
            run_id: started.run_id,
        }),
    ))
}

/// Starts the agent with its connectors. When the API exchanges tokens, the
/// caller's token goes to the agent's own client, for the run to act for them.
pub(crate) async fn start_with_connectors(
    state: &AppState,
    caller: &Caller,
    card_id: CardId,
    command: StartAgentRun,
) -> Result<capitainerie_core::StartedAgentRun, AgentRunError> {
    match &state.delegation {
        Some(delegation) => {
            state
                .usecase
                .start_agent_run_delegated(
                    caller.subject(),
                    card_id,
                    command,
                    RunDelegation {
                        exchanger: &delegation.exchanger,
                        issuer: &caller.organization.issuer_url,
                        user_token: &caller.token,
                    },
                )
                .await
        }
        None => {
            state
                .usecase
                .start_agent_run(caller.subject(), card_id, command)
                .await
        }
    }
}

/// Refuses what the agent and the organization lack before it asks GitHub
/// whether the installation reaches the repository and which branch to use.
pub(crate) async fn start_on_repository(
    state: &AppState,
    github: &GithubApps,
    subject: Subject,
    card_id: CardId,
    command: StartAgentRun,
    mut target: RepositoryTarget,
) -> Result<capitainerie_core::StartedAgentRun, RunAgentError> {
    state
        .usecase
        .check_agent_repository_run(subject.clone(), card_id, command, target.repository.clone())
        .await?;
    let organization_id = subject.organization_id;
    let not_connected = || RunAgentError::Refused(AgentRunRefusal::GithubNotConnected);
    if github.client(organization_id).await?.is_none() {
        return Err(not_connected());
    }
    let installation = state
        .usecase
        .github_installation(subject.clone())
        .await?
        .ok_or_else(not_connected)?;
    let reachable = github
        .repositories(organization_id, installation.installation_id)
        .await?;
    if !reaches(&reachable, &target.repository) {
        return Err(RunAgentError::Api(ApiError::Forbidden(Some(
            "repository_not_allowed".into(),
        ))));
    }
    if target.base_branch.is_none() {
        target.base_branch = Some(
            github
                .default_branch(
                    organization_id,
                    installation.installation_id,
                    &target.repository,
                )
                .await?,
        );
    }
    Ok(state
        .usecase
        .start_agent_repository_run(subject, card_id, command, target)
        .await?)
}

/// GitHub names are case-insensitive.
fn reaches(reachable: &[RepositoryRef], repository: &RepositoryRef) -> bool {
    reachable.iter().any(|candidate| {
        candidate.owner().eq_ignore_ascii_case(repository.owner())
            && candidate.name().eq_ignore_ascii_case(repository.name())
    })
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn rendered(error: impl Into<RunAgentError>) -> (StatusCode, serde_json::Value) {
        let response = error.into().into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn refusals_are_422_except_a_missing_connection_which_is_a_409_naming_the_slug() {
        let cases = [
            (
                AgentRunRefusal::NoAssignee,
                StatusCode::UNPROCESSABLE_ENTITY,
                "card_without_agent",
            ),
            (
                AgentRunRefusal::AgentArchived,
                StatusCode::UNPROCESSABLE_ENTITY,
                "agent_archived",
            ),
            (
                AgentRunRefusal::AgentPaused,
                StatusCode::UNPROCESSABLE_ENTITY,
                "agent_paused",
            ),
            (
                AgentRunRefusal::NoConnector,
                StatusCode::UNPROCESSABLE_ENTITY,
                "agent_without_connector",
            ),
            (
                AgentRunRefusal::NotConnected("mestier".into()),
                StatusCode::CONFLICT,
                "not_connected",
            ),
            (
                AgentRunRefusal::NoProvider,
                StatusCode::CONFLICT,
                "no_provider",
            ),
            (
                AgentRunRefusal::RepositoryNotGranted("acme/web".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
                "repository_not_granted",
            ),
            (
                AgentRunRefusal::GithubNotConnected,
                StatusCode::CONFLICT,
                "github_not_connected",
            ),
            (
                AgentRunRefusal::ExecutorDisabled("EXECUTOR_NAMESPACE".into()),
                StatusCode::CONFLICT,
                "executor_disabled",
            ),
        ];
        for (refusal, expected, code) in cases {
            let message = refusal.to_string();
            let (status, body) = rendered(AgentRunError::Refused(refusal)).await;
            assert_eq!(status, expected);
            assert_eq!(body["code"], code);
            assert_eq!(body["message"], message);
        }
    }

    #[tokio::test]
    async fn a_core_error_keeps_its_usual_mapping() {
        let (status, body) = rendered(CoreError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[test]
    fn writes_are_off_unless_asked() {
        let request: RunAgentRequest =
            serde_json::from_value(serde_json::json!({"expected_version": 2})).unwrap();
        assert!(!request.allow_writes);
        assert_eq!(request.repository, None);
        assert_eq!(request.base_branch, None);
    }

    #[test]
    fn the_reachable_check_ignores_case() {
        let reachable: Vec<RepositoryRef> = vec!["Acme/Web".parse().unwrap()];
        assert!(reaches(&reachable, &"acme/web".parse().unwrap()));
        assert!(!reaches(&reachable, &"acme/api".parse().unwrap()));
    }
}
