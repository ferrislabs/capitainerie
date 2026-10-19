use axum::{
    Extension, Json,
    extract::{Multipart, State, multipart::MultipartError},
    http::{HeaderMap, StatusCode, header::CONTENT_LENGTH},
};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::document::{
    MAX_PDF_BYTES, PdfDocument, ReadError, ports::DocumentReaderFactory,
};
use capitainerie_handlers::{ApiError, Caller};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{MULTIPART_OVERHEAD, documents_state::DocumentsState};

const FILE_FIELD: &str = "file";
const FILE_NAME_MAX_CHARS: usize = 120;

#[derive(TypedPath)]
#[typed_path("/api/v1/documents/text")]
pub struct DocumentTextPath;

#[derive(ToSchema)]
pub struct ReadDocumentRequest {
    /// The PDF file, 25 MiB at most.
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentTextResponse {
    /// The text of the PDF, to proofread before submitting.
    pub text: String,
    /// The id of the reader model that produced the text.
    pub model: String,
    /// The number of pages the reader returned.
    pub pages: usize,
}

#[utoipa::path(
    post,
    path = "/api/v1/documents/text",
    operation_id = "readDocument",
    tag = super::TAG,
    request_body(content = ReadDocumentRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "The PDF, read and not kept", body = DocumentTextResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents may not read documents", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "No reader is assigned (code reader_not_configured)", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 413, description = "The file is over 25 MiB", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 415, description = "The file is not a PDF", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "No file part, an unexpected part or an empty file", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 503, description = "Four PDFs are already being read", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 502, description = "The reader failed; its message is returned", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<R: DocumentReaderFactory + 'static>(
    _: DocumentTextPath,
    State(state): State<DocumentsState<R>>,
    Extension(caller): Extension<Caller>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<(StatusCode, Json<DocumentTextResponse>), ApiError> {
    let prepared = state
        .usecase
        .prepare_document_reading(caller.subject())
        .await
        .map_err(api_error)?;
    let _permit = state
        .permits
        .try_acquire()
        .map_err(|_| crate::busy("Trop de PDF en cours de lecture, réessayez dans un instant."))?;
    let declared = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|length| length > MAX_PDF_BYTES + MULTIPART_OVERHEAD) {
        return Err(too_large());
    }
    let document = read_document(multipart).await?;
    let reading = state
        .usecase
        .read_document(prepared, document, state.readers.as_ref())
        .await
        .map_err(api_error)?;
    Ok((
        StatusCode::OK,
        Json(DocumentTextResponse {
            text: reading.text,
            model: reading.model.model,
            pages: reading.pages,
        }),
    ))
}

fn api_error(error: ReadError) -> ApiError {
    match error {
        ReadError::NotConfigured => ApiError::ConflictCode {
            code: "reader_not_configured",
            message: "no PDF reader is assigned: choose one in Settings > Models".into(),
        },
        ReadError::Provider(message) => ApiError::BadGateway(message),
        ReadError::Core(error) => error.into(),
    }
}

async fn read_document(mut multipart: Multipart) -> Result<PdfDocument, ApiError> {
    let mut document = None;
    while let Some(field) = multipart.next_field().await.map_err(upload_error)? {
        if field.name() != Some(FILE_FIELD) || document.is_some() {
            return Err(ApiError::BadRequest("send a single part named file".into()));
        }
        let file_name = clean_file_name(field.file_name().unwrap_or_default());
        let content_type = field.content_type().map(str::to_owned);
        if !PdfDocument::declared_pdf(&file_name, content_type.as_deref()) {
            return Err(not_a_pdf());
        }
        let bytes = field.bytes().await.map_err(upload_error)?;
        if bytes.len() > MAX_PDF_BYTES {
            return Err(too_large());
        }
        if bytes.is_empty() {
            return Err(ApiError::BadRequest("the file is empty".into()));
        }
        document = Some(
            PdfDocument::accept(file_name, content_type.as_deref(), bytes)
                .map_err(|_| not_a_pdf())?,
        );
    }
    document.ok_or_else(|| ApiError::BadRequest("send a single part named file".into()))
}

fn clean_file_name(raw: &str) -> String {
    let name = raw.rsplit(['/', '\\']).next().unwrap_or_default();
    let name: String = name.chars().take(FILE_NAME_MAX_CHARS).collect();
    if name.is_empty() {
        "document.pdf".to_owned()
    } else {
        name
    }
}

fn too_large() -> ApiError {
    ApiError::PayloadTooLarge("the PDF is over 25 MiB".into())
}

fn not_a_pdf() -> ApiError {
    ApiError::UnsupportedMediaType("the file must be a PDF".into())
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
        assert_eq!(clean_file_name("C:\\Users\\a\\rapport.pdf"), "rapport.pdf");
        assert_eq!(clean_file_name("../../etc/passwd.pdf"), "passwd.pdf");
        assert_eq!(clean_file_name(""), "document.pdf");
        assert_eq!(clean_file_name(&"a".repeat(500)).chars().count(), 120);
    }
}
