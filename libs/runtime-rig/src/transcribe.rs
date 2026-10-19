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
        let response = request.send().await.map_err(|error| {
            CoreError::Internal(format!("the provider is unreachable: {error}"))
        })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            CoreError::Internal(format!("the transcription is unreadable: {error}"))
        })?;
        if !status.is_success() {
            return Err(CoreError::Internal(failure(status, &body)));
        }
        serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| value["text"].as_str().map(str::to_owned))
            .map(TranscriptionText)
            .ok_or_else(|| CoreError::Internal("the transcription is unreadable".into()))
    }
}

fn failure(status: reqwest::StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value["error"]["message"]
                .as_str()
                .or_else(|| value["message"].as_str())
                .or_else(|| value["detail"].as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| truncate_chars(body, ERROR_MAX_CHARS));
    let hint = match status.as_u16() {
        401 | 403 => " (check the API key)",
        _ => "",
    };
    format!(
        "the provider answered {status}{hint}: {}",
        truncate_chars(&detail, ERROR_MAX_CHARS)
    )
}
