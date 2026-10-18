use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use capitainerie_common::CoreError;
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
}

impl From<CoreError> for IntegrationError {
    fn from(error: CoreError) -> Self {
        Self::Api(error.into())
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
