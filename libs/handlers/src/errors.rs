use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use capitainerie_common::CoreError;
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("forbidden")]
    Forbidden(Option<String>),

    #[error("unavailable: {0}")]
    Unavailable(String),

    #[error("internal error")]
    Internal(String),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiErrorResponse {
    pub code: &'static str,
    pub status: u16,
    pub message: String,
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Forbidden(_) => "forbidden",
            Self::Unavailable(_) => "unavailable",
            Self::Internal(_) => "internal",
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::Conflict(message) => message.clone(),
            Self::Forbidden(Some(reason)) => reason.clone(),
            Self::Unavailable(_) => "unavailable".to_owned(),
            other => other.to_string(),
        }
    }
}

impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::NotFound => Self::NotFound,
            CoreError::Conflict(message) => Self::Conflict(message),
            CoreError::Forbidden { reason } => Self::Forbidden(reason),
            CoreError::Database(message) => Self::Unavailable(message),
            CoreError::Internal(message) => Self::Internal(message),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = ?self, "request failed");
        }
        let body = ApiErrorResponse {
            code: self.code(),
            status: status.as_u16(),
            message: self.public_message(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_map_to_http_statuses() {
        let cases = [
            (CoreError::NotFound, StatusCode::NOT_FOUND),
            (CoreError::Conflict("dup".into()), StatusCode::CONFLICT),
            (CoreError::Forbidden { reason: None }, StatusCode::FORBIDDEN),
            (
                CoreError::Database("down".into()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                CoreError::Internal("boom".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];
        for (core, status) in cases {
            assert_eq!(ApiError::from(core).into_response().status(), status);
        }
    }

    #[test]
    fn server_errors_do_not_leak_their_cause() {
        assert_eq!(
            ApiError::Internal("secret dsn".into()).public_message(),
            "internal error"
        );
        assert_eq!(
            ApiError::Unavailable("secret dsn".into()).public_message(),
            "unavailable"
        );
    }
}
