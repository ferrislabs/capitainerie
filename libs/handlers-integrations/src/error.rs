use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use capitainerie_common::CoreError;
use capitainerie_core::domain::integration::InstanceError;
use capitainerie_github::GithubError;
use capitainerie_handlers::{ApiError, errors::ApiErrorResponse};
use thiserror::Error;

use crate::signer::InvalidState;

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error("not configured: {message}")]
    NotConfigured {
        code: &'static str,
        message: &'static str,
    },

    #[error(transparent)]
    InvalidState(#[from] InvalidState),

    #[error("the connecting user has no access to this GitHub installation")]
    InstallationNotAccessible,

    #[error("the connecting user does not administer the account this installation belongs to")]
    InstallationNotOwned,

    #[error("the GitHub App lacks the organization permission Members: read")]
    MembersPermissionMissing,

    #[error("{message}")]
    Unprocessable { code: &'static str, message: String },
}

impl From<CoreError> for IntegrationError {
    fn from(error: CoreError) -> Self {
        Self::Api(error.into())
    }
}

impl From<InstanceError> for IntegrationError {
    fn from(error: InstanceError) -> Self {
        match error {
            InstanceError::Core(error) => error.into(),
            InstanceError::Connector(failure) => Self::Unprocessable {
                code: failure.code(),
                message: failure.to_string(),
            },
            InstanceError::NameTaken(_) => Self::Api(ApiError::ConflictCode {
                code: "instance_name_taken",
                message: error.to_string(),
            }),
        }
    }
}

impl From<GithubError> for IntegrationError {
    fn from(error: GithubError) -> Self {
        CoreError::from(error).into()
    }
}

impl IntoResponse for IntegrationError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Api(error) => return error.into_response(),
            Self::NotConfigured { code, message } => (
                StatusCode::SERVICE_UNAVAILABLE,
                code,
                format!("not configured on this server: {message}"),
            ),
            Self::InvalidState(error) => {
                (StatusCode::BAD_REQUEST, "invalid_state", error.to_string())
            }
            Self::InstallationNotAccessible => (
                StatusCode::FORBIDDEN,
                "installation_not_accessible",
                "the connecting user has no access to this GitHub installation".to_owned(),
            ),
            Self::MembersPermissionMissing => (
                StatusCode::FORBIDDEN,
                "github_app_members_permission_missing",
                "the GitHub App needs the organization permission Members: read to check who administers the organization"
                    .to_owned(),
            ),
            Self::Unprocessable { code, message } => {
                (StatusCode::UNPROCESSABLE_ENTITY, code, message)
            }
            Self::InstallationNotOwned => (
                StatusCode::FORBIDDEN,
                "installation_not_owned",
                "the connecting user does not administer the account this installation belongs to"
                    .to_owned(),
            ),
        };
        let body = ApiErrorResponse {
            code,
            status: status.as_u16(),
            message,
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use capitainerie_core::domain::integration::ConnectorFailure;

    use super::*;

    async fn rendered(error: impl Into<IntegrationError>) -> (StatusCode, serde_json::Value) {
        let response = error.into().into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn a_connector_failure_is_a_422_with_the_code_of_the_failure() {
        let cases = [
            (
                ConnectorFailure::Blocked("10.0.0.5".into()),
                "network_blocked",
            ),
            (
                ConnectorFailure::VersionMismatch("no JSON-2".into()),
                "version_mismatch",
            ),
            (
                ConnectorFailure::NotSupported("n8n 1.x".into()),
                "not_supported",
            ),
            (
                ConnectorFailure::Unreachable("refused".into()),
                "connection_failed",
            ),
            (ConnectorFailure::Timeout, "connection_failed"),
            (ConnectorFailure::Unauthorized, "connection_failed"),
            (
                ConnectorFailure::Protocol("not JSON".into()),
                "connection_failed",
            ),
            (
                ConnectorFailure::Remote {
                    status: 502,
                    message: "bad gateway".into(),
                },
                "connection_failed",
            ),
        ];
        for (failure, code) in cases {
            let message = failure.to_string();
            let (status, body) = rendered(InstanceError::Connector(failure)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], code);
            assert_eq!(body["status"], 422);
            assert_eq!(body["message"], message);
        }
    }

    #[tokio::test]
    async fn a_taken_name_is_a_409_and_a_core_error_keeps_its_usual_mapping() {
        let (status, body) = rendered(InstanceError::NameTaken("Odoo prod".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "instance_name_taken");
        assert!(body["message"].as_str().unwrap().contains("Odoo prod"));

        let cases = [
            (CoreError::NotFound, StatusCode::NOT_FOUND, "not_found"),
            (
                CoreError::Invalid("empty".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid",
            ),
            (
                CoreError::Forbidden { reason: None },
                StatusCode::FORBIDDEN,
                "forbidden",
            ),
            (
                CoreError::Internal("SECRETS_KEY is not configured".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
            ),
        ];
        for (error, expected, code) in cases {
            let (status, body) = rendered(InstanceError::Core(error)).await;
            assert_eq!(status, expected);
            assert_eq!(body["code"], code);
        }
    }
}
