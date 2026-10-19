use axum::{
    http::{Method, StatusCode, header::WWW_AUTHENTICATE},
    response::{IntoResponse, Response},
};
use capitainerie_common::RunId;

use crate::credentials::CredentialsError;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("unauthorized: {reason}")]
    Unauthorized { reason: &'static str },
    #[error("forbidden: {reason}")]
    Forbidden { run_id: RunId, reason: &'static str },
    #[error("request body too large")]
    TooLarge { run_id: RunId },
    #[error("too many requests in flight")]
    TooManyRequests { run_id: RunId },
    #[error("credentials backend failed: {0}")]
    Credentials(#[from] CredentialsError),
    #[error("upstream unreachable: {0}")]
    Upstream(String),
}

impl ProxyError {
    pub fn forbidden(run_id: RunId, reason: &'static str) -> Self {
        Self::Forbidden { run_id, reason }
    }

    pub fn log(&self, method: &Method, class: &str) {
        match self {
            Self::Unauthorized { reason } => {
                tracing::warn!(%method, class, reason, "request denied")
            }
            Self::Forbidden { run_id, reason } => {
                tracing::warn!(run_id = %run_id, %method, class, reason, "request denied")
            }
            Self::TooLarge { run_id } => {
                tracing::warn!(run_id = %run_id, %method, class, reason = "body too large", "request denied")
            }
            Self::TooManyRequests { run_id } => {
                tracing::warn!(run_id = %run_id, %method, class, reason = "too many requests in flight", "request denied")
            }
            Self::Credentials(error) => tracing::error!(%method, class, %error, "lookup failed"),
            Self::Upstream(error) => tracing::error!(%method, class, %error, "upstream failed"),
        }
    }
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        match self {
            // git sends credentials only after a Basic challenge.
            Self::Unauthorized { .. } => (
                StatusCode::UNAUTHORIZED,
                [(WWW_AUTHENTICATE, r#"Basic realm="capitainerie""#)],
                "unauthorized",
            )
                .into_response(),
            Self::Forbidden { reason, .. } => (StatusCode::FORBIDDEN, reason).into_response(),
            Self::TooLarge { .. } => {
                (StatusCode::PAYLOAD_TOO_LARGE, "request body too large").into_response()
            }
            Self::TooManyRequests { .. } => {
                (StatusCode::TOO_MANY_REQUESTS, "too many requests in flight").into_response()
            }
            Self::Credentials(_) => {
                (StatusCode::BAD_GATEWAY, "credentials unavailable").into_response()
            }
            Self::Upstream(_) => (StatusCode::BAD_GATEWAY, "upstream unavailable").into_response(),
        }
    }
}
