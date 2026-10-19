use axum::{
    Extension, Json,
    extract::{Multipart, State, multipart::MultipartError},
    http::{HeaderMap, StatusCode, header::CONTENT_LENGTH},
};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::transcription::{
    AudioClip, AudioMime, MAX_AUDIO_BYTES, TranscribeError, ports::TranscriberFactory,
};
use capitainerie_handlers::{ApiError, Caller};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{MULTIPART_OVERHEAD, state::TranscriptionsState};

const FILE_FIELD: &str = "file";
const FILE_NAME_MAX_CHARS: usize = 120;

#[derive(TypedPath)]
#[typed_path("/api/v1/transcriptions")]
pub struct TranscriptionsPath;

#[derive(ToSchema)]
pub struct CreateTranscriptionRequest {
    /// The audio file: mp3, m4a, wav, webm, ogg or aac, 25 MiB at most.
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TranscriptionResponse {
    /// The transcribed text, to proofread before submitting.
    pub text: String,
    /// The id of the transcriber model that produced the text.
    pub model: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/transcriptions",
    operation_id = "createTranscription",
    tag = super::TAG,
    request_body(content = CreateTranscriptionRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "The audio, transcribed and not kept", body = TranscriptionResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents may not transcribe", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "No transcriber is assigned (code transcriber_not_configured)", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 413, description = "The file is over 25 MiB", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 415, description = "The format is not mp3, m4a, wav, webm, ogg or aac", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "No file part, an unexpected part or an empty file", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 502, description = "The provider failed; its message is returned", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<F: TranscriberFactory + 'static>(
    _: TranscriptionsPath,
    State(state): State<TranscriptionsState<F>>,
    Extension(caller): Extension<Caller>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<(StatusCode, Json<TranscriptionResponse>), ApiError> {
    let prepared = state
        .usecase
        .prepare_transcription(caller.subject())
        .await
        .map_err(api_error)?;
    let declared = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|length| length > MAX_AUDIO_BYTES + MULTIPART_OVERHEAD) {
        return Err(too_large());
    }
    let audio = read_clip(multipart).await?;
    let transcription = state
        .usecase
        .transcribe(prepared, audio, state.factory.as_ref())
        .await
        .map_err(api_error)?;
    Ok((
        StatusCode::OK,
        Json(TranscriptionResponse {
            text: transcription.text,
            model: transcription.model.model,
        }),
    ))
}

fn api_error(error: TranscribeError) -> ApiError {
    match error {
        TranscribeError::NotConfigured => ApiError::ConflictCode {
            code: "transcriber_not_configured",
            message: "no transcriber is assigned: choose one in Settings > Models".into(),
        },
        TranscribeError::Provider(message) => ApiError::BadGateway(message),
        TranscribeError::Core(error) => error.into(),
    }
}

async fn read_clip(mut multipart: Multipart) -> Result<AudioClip, ApiError> {
    let mut clip = None;
    while let Some(field) = multipart.next_field().await.map_err(upload_error)? {
        if field.name() != Some(FILE_FIELD) || clip.is_some() {
            return Err(ApiError::BadRequest("send a single part named file".into()));
        }
        let file_name = clean_file_name(field.file_name().unwrap_or_default());
        let mime = AudioMime::from_file_name(&file_name)
            .or_else(|| field.content_type().and_then(AudioMime::from_content_type))
            .ok_or_else(|| {
                ApiError::UnsupportedMediaType(
                    "the format must be mp3, m4a, wav, webm, ogg or aac".into(),
                )
            })?;
        let bytes = field.bytes().await.map_err(upload_error)?;
        if bytes.len() > MAX_AUDIO_BYTES {
            return Err(too_large());
        }
        if bytes.is_empty() {
            return Err(ApiError::BadRequest("the file is empty".into()));
        }
        clip = Some(AudioClip {
            file_name,
            mime,
            bytes,
        });
    }
    clip.ok_or_else(|| ApiError::BadRequest("send a single part named file".into()))
}

fn clean_file_name(raw: &str) -> String {
    let name = raw.rsplit(['/', '\\']).next().unwrap_or_default();
    let name: String = name.chars().take(FILE_NAME_MAX_CHARS).collect();
    if name.is_empty() {
        "audio".to_owned()
    } else {
        name
    }
}

fn too_large() -> ApiError {
    ApiError::PayloadTooLarge("the audio file is over 25 MiB".into())
}

fn upload_error(error: MultipartError) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        too_large()
    } else {
        ApiError::BadRequest("the multipart body is invalid".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_name_loses_its_directories_and_is_bounded() {
        assert_eq!(clean_file_name("C:\\Users\\a\\réunion.mp3"), "réunion.mp3");
        assert_eq!(clean_file_name("../../etc/passwd.ogg"), "passwd.ogg");
        assert_eq!(clean_file_name(""), "audio");
        assert_eq!(clean_file_name(&"a".repeat(500)).chars().count(), 120);
    }
}
