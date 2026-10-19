use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    provider::{ModelRef, ProviderAccess, ProviderKind},
    transcription::{
        AudioClip, TranscriptionText,
        ports::{Transcriber, TranscriberFactory},
    },
};
use rig_reqwest::reqwest::{
    self, Body,
    multipart::{Form, Part},
};
use serde_json::Value;

use crate::{
    client::{MISTRAL_BASE, http_client},
    error::RuntimeError,
    usage::truncate_chars,
};

const TIMEOUT_SECS: u64 = 300;
const ERROR_MAX_CHARS: usize = 200;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const UNREACHABLE: &str = "Le fournisseur de transcription est injoignable.";
const UNREADABLE: &str = "La réponse du fournisseur de transcription est illisible.";

#[derive(Clone)]
pub struct RigTranscribers {
    http: reqwest::Client,
}

pub struct RigTranscriber {
    http: reqwest::Client,
    kind: ProviderKind,
    base_url: Option<String>,
    key: SecretToken,
}

impl RigTranscribers {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            http: http_client(TIMEOUT_SECS)?,
        })
    }
}

impl TranscriberFactory for RigTranscribers {
    type Bound = RigTranscriber;

    fn bind(&self, access: &ProviderAccess) -> RigTranscriber {
        RigTranscriber {
            http: self.http.clone(),
            kind: access.kind,
            base_url: access.base_url.clone(),
            key: access.secret.clone(),
        }
    }
}

impl RigTranscriber {
    fn url(&self) -> Result<String, CoreError> {
        let base = self
            .base_url
            .as_deref()
            .map(|url| url.trim_end_matches('/'));
        match self.kind {
            ProviderKind::Mistral => Ok(format!(
                "{}/v1/audio/transcriptions",
                base.unwrap_or(MISTRAL_BASE)
            )),
            ProviderKind::OpenAiCompatible => base
                .map(|base| format!("{base}/audio/transcriptions"))
                .ok_or_else(|| {
                    CoreError::Invalid(
                        "the OpenAI-compatible provider has no base url configured".into(),
                    )
                }),
            ProviderKind::Anthropic => Err(CoreError::Invalid(
                "anthropic offers no transcription model".into(),
            )),
        }
    }
}

impl Transcriber for RigTranscriber {
    async fn transcribe(
        &self,
        model: &ModelRef,
        audio: AudioClip,
    ) -> Result<TranscriptionText, CoreError> {
        let url = self.url()?;
        let length = audio.bytes.len() as u64;
        let file = Part::stream_with_length(Body::from(audio.bytes), length)
            .file_name(audio.file_name)
            .mime_str(audio.mime.as_str())
            .map_err(|error| CoreError::Internal(format!("the audio type is invalid: {error}")))?;
        let form = Form::new()
            .text("model", model.model.clone())
            .part("file", file);
        let mut request = self.http.post(url).multipart(form);
        if !self.key.expose().is_empty() {
            request = request.bearer_auth(self.key.expose());
        }
        let mut response = request.send().await.map_err(|error| {
            tracing::warn!(error = %error.without_url(), "transcription provider unreachable");
            CoreError::Internal(UNREACHABLE.into())
        })?;
        let status = response.status();
        let body = read_capped(&mut response).await?;
        if !status.is_success() {
            return Err(CoreError::Internal(failure(status, &body)));
        }
        serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|value| value["text"].as_str().map(str::to_owned))
            .map(TranscriptionText)
            .ok_or_else(|| CoreError::Internal(UNREADABLE.into()))
    }
}

async fn read_capped(response: &mut reqwest::Response) -> Result<Vec<u8>, CoreError> {
    let too_large =
        || CoreError::Internal("La réponse du fournisseur est trop volumineuse.".into());
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
                tracing::warn!(error = %error.without_url(), "transcription answer unreadable");
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
                .map(|message| truncate_chars(message, ERROR_MAX_CHARS))
        });
    match detail {
        Some(detail) => format!("Le fournisseur de transcription a répondu {status} : {detail}"),
        None => format!("Le fournisseur de transcription a répondu {status}."),
    }
}
