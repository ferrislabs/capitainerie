use axum::{
    Json,
    http::{HeaderValue, StatusCode, header::WWW_AUTHENTICATE},
    response::{IntoResponse, Response},
};
use capitainerie_auth::AuthError;
use capitainerie_common::CoreError;
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("conflict: {message}")]
    ConflictCode { code: &'static str, message: String },

    #[error("payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("unsupported media type: {0}")]
    UnsupportedMediaType(String),

    #[error("bad gateway: {0}")]
    BadGateway(String),

    #[error("forbidden")]
    Forbidden(Option<String>),

    #[error("busy: {0}")]
    Busy(String),

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
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) | Self::ConflictCode { .. } => StatusCode::CONFLICT,
            Self::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::UnsupportedMediaType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Unavailable(_) | Self::Busy(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "unauthorized",
            Self::BadRequest(_) => "invalid",
            Self::NotFound => "not_found",
            Self::Conflict(_) => "conflict",
            Self::ConflictCode { code, .. } => code,
            Self::PayloadTooLarge(_) => "payload_too_large",
            Self::UnsupportedMediaType(_) => "unsupported_media_type",
            Self::BadGateway(_) => "bad_gateway",
            Self::Forbidden(_) => "forbidden",
            Self::Unavailable(_) => "unavailable",
            Self::Busy(_) => "busy",
            Self::Internal(_) => "internal",
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::Unauthorized(_) => "unauthorized".to_owned(),
            Self::BadRequest(message)
            | Self::Conflict(message)
            | Self::PayloadTooLarge(message)
            | Self::UnsupportedMediaType(message)
            | Self::BadGateway(message)
            | Self::Busy(message)
            | Self::ConflictCode { message, .. } => message.clone(),
            Self::Forbidden(Some(reason)) => reason.clone(),
            Self::Unavailable(_) => "unavailable".to_owned(),
            other => other.to_string(),
        }
    }
}

impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Invalid(message) => Self::BadRequest(message),
            CoreError::NotFound => Self::NotFound,
            CoreError::Conflict(message) => Self::Conflict(message),
            CoreError::Forbidden { reason } => Self::Forbidden(reason),
            CoreError::Database(message) => Self::Unavailable(message),
            CoreError::Internal(message) => Self::Internal(message),
        }
    }
}

impl From<AuthError> for ApiError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::Discovery(message) => Self::Unavailable(message),
            other => Self::Unauthorized(other.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = ?self, "request failed");
        }
        if let Self::Unauthorized(reason) = &self {
            tracing::info!(%reason, "request rejected");
        }
        let body = ApiErrorResponse {
            code: self.code(),
            status: status.as_u16(),
            message: self.public_message(),
        };
        let mut response = (status, Json(body)).into_response();
        if status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                WWW_AUTHENTICATE,
                HeaderValue::from_static(r#"Bearer error="invalid_token""#),
            );
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_map_to_http_statuses() {
        let cases = [
            (
                CoreError::Invalid("empty".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
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
    fn upload_and_gateway_errors_have_their_own_status_and_code() {
        let cases = [
            (
                ApiError::PayloadTooLarge("big".into()),
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
            ),
            (
                ApiError::UnsupportedMediaType("pdf".into()),
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
            ),
            (
                ApiError::BadGateway("down".into()),
                StatusCode::BAD_GATEWAY,
                "bad_gateway",
            ),
            (
                ApiError::Busy("later".into()),
                StatusCode::SERVICE_UNAVAILABLE,
                "busy",
            ),
            (
                ApiError::ConflictCode {
                    code: "transcriber_not_configured",
                    message: "none".into(),
                },
                StatusCode::CONFLICT,
                "transcriber_not_configured",
            ),
        ];
        for (error, status, code) in cases {
            assert_eq!(error.code(), code);
            assert_eq!(error.into_response().status(), status);
        }
    }

    #[test]
    fn unauthorized_carries_a_bearer_challenge_without_its_reason() {
        let response = ApiError::Unauthorized("kid abc unknown".into()).into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(
            response.headers()[WWW_AUTHENTICATE]
                .to_str()
                .unwrap()
                .starts_with("Bearer")
        );
        assert_eq!(
            ApiError::Unauthorized("kid abc unknown".into()).public_message(),
            "unauthorized"
        );
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
