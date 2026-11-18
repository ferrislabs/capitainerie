use axum::{
    Json,
    body::Bytes,
    extract::{State, rejection::BytesRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::SourceEventId;
use capitainerie_core::domain::source::{PAYLOAD_MAX_BYTES, check_payload, service::HookRefusal};
use capitainerie_handlers::{ApiError, AppState, errors::ApiErrorResponse};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;

#[derive(TypedPath, Deserialize)]
#[typed_path("/hooks/sources/{token}")]
pub struct HookPath {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HookResponse {
    pub event_id: SourceEventId,
}

#[derive(Debug)]
pub enum HookError {
    Api(ApiError),
    Refused(HookRefusal),
}

impl From<ApiError> for HookError {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

impl From<capitainerie_common::CoreError> for HookError {
    fn from(error: capitainerie_common::CoreError) -> Self {
        Self::Api(error.into())
    }
}

impl IntoResponse for HookError {
    fn into_response(self) -> Response {
        match self {
            Self::Api(error) => error.into_response(),
            Self::Refused(refusal) => {
                let status = StatusCode::TOO_MANY_REQUESTS;
                let body = ApiErrorResponse {
                    code: match refusal {
                        HookRefusal::RateLimited => "rate_limited",
                    },
                    status: status.as_u16(),
                    message: refusal.to_string(),
                };
                (status, [("retry-after", "60")], Json(body)).into_response()
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/hooks/sources/{token}",
    operation_id = "receiveSourceEvent",
    tag = super::TAG,
    params(("token" = String, Path, description = "The source's webhook token")),
    request_body(
        content = Object,
        description = "Any JSON (or plain text, kept as `text`) of at most 256 KiB. Its `title`, `subject` or the first line of its `text` titles the card; the whole payload becomes the card's description, as untrusted data",
    ),
    responses(
        (status = 202, description = "The event is stored, its card created and, when the source asks for it, its agent run queued", body = HookResponse),
        (status = 404, description = "No enabled source has this token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 413, description = "The body exceeds 256 KiB", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The body is neither JSON nor UTF-8 text", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 429, description = "The source received too many events in the last minute", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
)]
pub async fn handler(
    HookPath { token }: HookPath,
    State(state): State<AppState>,
    body: Result<Bytes, BytesRejection>,
) -> Result<(StatusCode, Json<HookResponse>), HookError> {
    let payload = payload(body)?;
    let event = state
        .usecase
        .receive_source_event(&token, payload)
        .await?
        .map_err(HookError::Refused)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(HookResponse { event_id: event.id }),
    ))
}

fn too_large() -> ApiError {
    ApiError::PayloadTooLarge(format!("the body exceeds {PAYLOAD_MAX_BYTES} bytes"))
}

fn payload(body: Result<Bytes, BytesRejection>) -> Result<Value, ApiError> {
    let body = body.map_err(|rejection| {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            too_large()
        } else {
            ApiError::BadRequest(rejection.body_text())
        }
    })?;
    if body.len() > PAYLOAD_MAX_BYTES {
        return Err(too_large());
    }
    let value = if body.iter().all(u8::is_ascii_whitespace) {
        json!({})
    } else if let Ok(value) = serde_json::from_slice::<Value>(&body) {
        value
    } else {
        let text = std::str::from_utf8(&body)
            .map_err(|_| ApiError::BadRequest("the body is neither JSON nor UTF-8 text".into()))?;
        json!({ "text": text })
    };
    check_payload(&value).map_err(|_| too_large())?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_kept_text_is_wrapped_and_an_empty_body_is_an_empty_object() {
        assert_eq!(
            payload(Ok(Bytes::from_static(br#"{"title":"x"}"#))).unwrap(),
            json!({"title": "x"})
        );
        assert_eq!(
            payload(Ok(Bytes::from_static(b"Bonjour\nSuite"))).unwrap(),
            json!({"text": "Bonjour\nSuite"})
        );
        assert_eq!(payload(Ok(Bytes::from_static(b" \n"))).unwrap(), json!({}));
        assert!(matches!(
            payload(Ok(Bytes::from_static(&[0xff, 0xfe]))),
            Err(ApiError::BadRequest(_))
        ));
        let quotes = "\"".repeat(PAYLOAD_MAX_BYTES - 10);
        assert!(matches!(
            payload(Ok(Bytes::from(quotes))),
            Err(ApiError::PayloadTooLarge(_))
        ));
    }
}
