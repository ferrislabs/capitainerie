use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    document::{
        DocumentText, PdfDocument,
        ports::{DocumentReader, DocumentReaderFactory},
    },
    provider::{ModelRef, ProviderAccess, ProviderKind},
};
use rig_reqwest::reqwest::{self, header::CONTENT_TYPE};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    client::{MISTRAL_BASE, http_client},
    error::RuntimeError,
    usage::truncate_chars,
};

const TIMEOUT_SECS: u64 = 300;
const ERROR_MAX_CHARS: usize = 200;
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const BODY_PREFIX: &str =
    ",\"document\":{\"type\":\"document_url\",\"document_url\":\"data:application/pdf;base64,";
const BODY_SUFFIX: &str = "\"}}";
const UNREACHABLE: &str = "Le service de lecture des PDF est injoignable.";
const UNREADABLE: &str = "La réponse du service de lecture des PDF est illisible.";
const TOO_LARGE: &str = "La réponse du service de lecture des PDF est trop volumineuse.";

#[derive(Clone)]
pub struct RigDocumentReaders {
    http: reqwest::Client,
}

pub struct RigDocumentReader {
    http: reqwest::Client,
    kind: ProviderKind,
    base_url: Option<String>,
    key: SecretToken,
}

#[derive(Deserialize)]
struct OcrResponse {
    pages: Vec<OcrPage>,
}

#[derive(Deserialize)]
struct OcrPage {
    index: Option<u64>,
    markdown: String,
}

impl RigDocumentReaders {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            http: http_client(TIMEOUT_SECS)?,
        })
    }
}

impl DocumentReaderFactory for RigDocumentReaders {
    type Bound = RigDocumentReader;

    fn bind(&self, access: &ProviderAccess) -> RigDocumentReader {
        RigDocumentReader {
            http: self.http.clone(),
            kind: access.kind,
            base_url: access.base_url.clone(),
            key: access.secret.clone(),
        }
    }
}

impl RigDocumentReader {
    fn url(&self) -> Result<String, CoreError> {
        match self.kind {
            ProviderKind::Mistral => Ok(format!(
                "{}/v1/ocr",
                self.base_url
                    .as_deref()
                    .map_or(MISTRAL_BASE, |url| url.trim_end_matches('/'))
            )),
            ProviderKind::Anthropic | ProviderKind::OpenAiCompatible => Err(CoreError::Invalid(
                "only mistral offers a document reader".into(),
            )),
        }
    }
}

impl DocumentReader for RigDocumentReader {
    async fn read(
        &self,
        model: &ModelRef,
        document: PdfDocument,
    ) -> Result<DocumentText, CoreError> {
        let url = self.url()?;
        let body = request_body(&model.model, &document)?;
        drop(document);
        let mut response = self
            .http
            .post(url)
            .bearer_auth(self.key.expose())
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(error = %error.without_url(), "document reader unreachable");
                CoreError::Internal(UNREACHABLE.into())
            })?;
        let status = response.status();
        let body = read_capped(&mut response).await?;
        if !status.is_success() {
            return Err(CoreError::Internal(failure(status, &body)));
        }
        let parsed = serde_json::from_slice::<OcrResponse>(&body)
            .map_err(|_| CoreError::Internal(UNREADABLE.into()))?;
        Ok(join_pages(parsed.pages))
    }
}

fn request_body(model: &str, document: &PdfDocument) -> Result<Vec<u8>, CoreError> {
    let build_error = || CoreError::Internal("the OCR request could not be built".into());
    let mut body = Vec::with_capacity(BODY_PREFIX.len() + document.bytes.len() * 4 / 3 + 256);
    body.extend_from_slice(b"{\"model\":");
    serde_json::to_writer(&mut body, model).map_err(|_| build_error())?;
    body.extend_from_slice(BODY_PREFIX.as_bytes());
    document.append_base64(&mut body)?;
    body.extend_from_slice(BODY_SUFFIX.as_bytes());
    Ok(body)
}

fn join_pages(pages: Vec<OcrPage>) -> DocumentText {
    let count = pages.len();
    let indexed = pages.iter().all(|page| page.index.is_some());
    let mut ordered: Vec<(u64, OcrPage)> = pages
        .into_iter()
        .enumerate()
        .map(|(position, page)| (page.index.unwrap_or(position as u64), page))
        .collect();
    if indexed {
        ordered.sort_by_key(|(index, _)| *index);
    }
    let text = ordered
        .into_iter()
        .map(|(_, page)| page.markdown)
        .collect::<Vec<_>>()
        .join("\n\n");
    DocumentText { text, pages: count }
}

async fn read_capped(response: &mut reqwest::Response) -> Result<Vec<u8>, CoreError> {
    let too_large = || CoreError::Internal(TOO_LARGE.into());
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(too_large());
    }
    let mut body = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
                    return Err(too_large());
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => return Ok(body),
            Err(error) => {
                tracing::warn!(error = %error.without_url(), "document reader answer unreadable");
                return Err(CoreError::Internal(UNREADABLE.into()));
            }
        }
    }
}

fn failure(status: reqwest::StatusCode, body: &[u8]) -> String {
    let detail = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            value["error"]["message"]
                .as_str()
                .or_else(|| value["message"].as_str())
                .or_else(|| value["detail"].as_str())
                .map(|message| {
                    let printable: String = message.chars().filter(|c| !c.is_control()).collect();
                    truncate_chars(&printable, ERROR_MAX_CHARS)
                })
        });
    match detail {
        Some(detail) => format!("Le service de lecture des PDF a répondu {status} : {detail}"),
        None => format!("Le service de lecture des PDF a répondu {status}."),
    }
}
