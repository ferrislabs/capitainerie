use std::sync::Arc;

use axum::{Extension, Json, extract::State, http::StatusCode};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_core::{AgentDelegation, domain::execution::ports::CodeHost};
use capitainerie_github::GithubClient;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::response::CardResponse;

/// `None` when the server has no GitHub App.
#[derive(Clone, Default)]
pub struct CodeHostHandle(pub Option<Arc<GithubClient>>);

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}/executions")]
pub struct ExecuteCardPath {
    pub card_id: CardId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecuteCardRequest {
    /// `owner/name`, one of the repositories of the organization's installation.
    pub repository: String,
    /// The version the caller last read; a stale one is refused with 409.
    pub expected_version: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExecuteCardResponse {
    pub run_id: RunId,
    pub card: CardResponse,
}

#[utoipa::path(
    post,
    path = "/api/v1/cards/{card_id}/executions",
    operation_id = "executeCard",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    request_body = ExecuteCardRequest,
    responses(
        (status = 202, description = "The card is In progress and an Execute run is queued", body = ExecuteCardResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused, or repository_not_allowed: the installation does not reach that repository", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "card_not_executable, execution_already_active, github_not_connected, or the card changed since it was read", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Not an owner/name repository", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ExecuteCardPath { card_id }: ExecuteCardPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(CodeHostHandle(github)): Extension<CodeHostHandle>,
    Json(request): Json<ExecuteCardRequest>,
) -> Result<(StatusCode, Json<ExecuteCardResponse>), ApiError> {
    let repository = request.repository.parse()?;
    let subject = caller.subject();
    let not_connected = || CoreError::Conflict("github_not_connected".into());
    let github = github.ok_or_else(not_connected)?;
    let installation = state
        .usecase
        .github_installation(subject.clone())
        .await?
        .ok_or_else(not_connected)?;
    let reachable = github.repositories(installation.installation_id).await?;
    if !reachable.contains(&repository) {
        return Err(ApiError::Forbidden(Some("repository_not_allowed".into())));
    }
    let base_branch = github
        .default_branch(installation.installation_id, &repository)
        .await
        .map_err(CoreError::from)?;
    let delegation = state.delegation.as_ref().ok_or_else(|| {
        ApiError::Unavailable("executions need DELEGATION_KEY and the exchange client".into())
    })?;
    let started = state
        .usecase
        .execute_card_delegated(
            subject,
            card_id,
            repository,
            base_branch,
            request.expected_version,
            AgentDelegation {
                exchanger: &delegation.exchanger,
                issuer: &caller.organization.issuer_url,
                user_token: &caller.token,
                agents: &[],
            },
        )
        .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(ExecuteCardResponse {
            run_id: started.run_id,
            card: started.card.into(),
        }),
    ))
}
