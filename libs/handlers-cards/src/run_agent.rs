use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_core::domain::agent_task::{AgentRunError, AgentRunRefusal, StartAgentRun};
use capitainerie_handlers::{ApiError, AppState, Caller, errors::ApiErrorResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}/agent-runs")]
pub struct RunAgentPath {
    pub card_id: CardId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RunAgentRequest {
    pub expected_version: i32,
    #[serde(default)]
    pub allow_writes: bool,
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
                    AgentRunRefusal::NotConnected(_) | AgentRunRefusal::NoProvider => {
                        StatusCode::CONFLICT
                    }
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
        (status = 202, description = "A run of the card's assignee agent is queued", body = RunAgentResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused: an agent cannot start an agent run", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "not_connected (the message names the connector slug), no_provider (neither the caller nor the organization connected an LLM provider), execution_already_active, or the card changed since it was read", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "card_without_agent, agent_archived or agent_without_connector", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    RunAgentPath { card_id }: RunAgentPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<RunAgentRequest>,
) -> Result<(StatusCode, Json<RunAgentResponse>), RunAgentError> {
    let started = state
        .usecase
        .start_agent_run(
            caller.subject(),
            card_id,
            StartAgentRun {
                expected_version: request.expected_version,
                allow_writes: request.allow_writes,
            },
        )
        .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(RunAgentResponse {
            run_id: started.run_id,
        }),
    ))
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
    }
}
