use std::sync::Arc;

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::Usage,
    provider::{ModelTarget, ProviderKind},
    similarity::{Embedding, ports::Embedder},
};
use rig_reqwest::reqwest;
use serde::Deserialize;
use serde_json::json;
use tracing::{Instrument, field, info_span};

use crate::{
    args::LlmArgs,
    client::{MISTRAL_BASE, http_client, provider_name},
    error::RuntimeError,
};

const UNUSED_KEY: &str = "unused";

#[derive(Clone)]
pub struct RigEmbedder {
    http: reqwest::Client,
    default_url: Arc<str>,
    default_key: Arc<str>,
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingItem>,
    usage: Option<EmbeddingsUsage>,
}

#[derive(Deserialize)]
struct EmbeddingItem {
    index: Option<usize>,
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingsUsage {
    prompt_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

struct Endpoint {
    url: String,
    key: String,
    provider: &'static str,
}

impl RigEmbedder {
    pub fn new(llm: &LlmArgs) -> Result<Self, RuntimeError> {
        Ok(Self {
            http: http_client(llm.timeout_secs)?,
            default_url: Arc::from(format!("{}/embeddings", llm.base_url.trim_end_matches('/'))),
            default_key: Arc::from(llm.api_key.as_str()),
        })
    }

    fn endpoint(&self, target: &ModelTarget) -> Result<Endpoint, CoreError> {
        let Some(access) = &target.access else {
            return match target.model.provider {
                ProviderKind::OpenAiCompatible => Ok(Endpoint {
                    url: self.default_url.to_string(),
                    key: self.default_key.to_string(),
                    provider: "openai",
                }),
                kind => Err(CoreError::Invalid(format!(
                    "no {} key configured for this organization",
                    kind.as_str()
                ))),
            };
        };
        let base_url = access
            .base_url
            .as_deref()
            .map(|url| url.trim_end_matches('/'));
        let key = match access.secret.expose() {
            "" => UNUSED_KEY.to_owned(),
            key => key.to_owned(),
        };
        let provider = provider_name(access.kind);
        match access.kind {
            ProviderKind::OpenAiCompatible => {
                let base_url = base_url.ok_or_else(|| {
                    CoreError::Invalid(
                        "the OpenAI-compatible provider has no base url configured".into(),
                    )
                })?;
                Ok(Endpoint {
                    url: format!("{base_url}/embeddings"),
                    key,
                    provider,
                })
            }
            ProviderKind::Mistral => Ok(Endpoint {
                url: format!("{}/v1/embeddings", base_url.unwrap_or(MISTRAL_BASE)),
                key,
                provider,
            }),
            ProviderKind::Anthropic => Err(CoreError::Invalid(
                "Anthropic offers no embeddings API: choose an embedding model from another provider"
                    .into(),
            )),
        }
    }

    async fn request(
        &self,
        endpoint: &Endpoint,
        model: &str,
        texts: &[String],
    ) -> Result<(Vec<Embedding>, Usage), CoreError> {
        let response = self
            .http
            .post(&endpoint.url)
            .bearer_auth(&endpoint.key)
            .json(&json!({ "model": model, "input": texts }))
            .send()
            .await
            .map_err(|error| CoreError::Internal(format!("the embedding call failed: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let message = format!(
                "the embedding call returned {status}: {}",
                crate::usage::truncate_chars(&body, 300)
            );
            return Err(if status.is_client_error() && status.as_u16() != 429 {
                CoreError::Invalid(message)
            } else {
                CoreError::Internal(message)
            });
        }
        let parsed: EmbeddingsResponse = response.json().await.map_err(|error| {
            CoreError::Internal(format!("the embedding response is unreadable: {error}"))
        })?;
        let tokens = parsed
            .usage
            .and_then(|usage| usage.prompt_tokens.or(usage.total_tokens))
            .unwrap_or(0);
        let mut items = parsed.data;
        if items.len() != texts.len() {
            return Err(CoreError::Internal(format!(
                "the embedding call returned {} vectors for {} texts",
                items.len(),
                texts.len()
            )));
        }
        items.sort_by_key(|item| item.index.unwrap_or(0));
        let width = items.first().map_or(0, |item| item.embedding.len());
        if width == 0 || items.iter().any(|item| item.embedding.len() != width) {
            return Err(CoreError::Internal(format!(
                "model {model} returned vectors of inconsistent or null size"
            )));
        }
        Ok((
            items
                .into_iter()
                .map(|item| Embedding(item.embedding))
                .collect(),
            Usage {
                model: model.to_owned(),
                input_tokens: tokens,
                output_tokens: 0,
            },
        ))
    }
}

impl Embedder for RigEmbedder {
    async fn embed(
        &self,
        target: &ModelTarget,
        texts: &[String],
    ) -> Result<(Vec<Embedding>, Usage), CoreError> {
        let model = target.model.model.as_str();
        if texts.is_empty() {
            return Ok((
                Vec::new(),
                Usage {
                    model: model.to_owned(),
                    ..Usage::default()
                },
            ));
        }
        let endpoint = self.endpoint(target)?;
        let span = info_span!(
            "embeddings",
            otel.name = format!("embeddings {model}"),
            gen_ai.operation.name = "embeddings",
            gen_ai.provider.name = endpoint.provider,
            gen_ai.request.model = model,
            gen_ai.usage.input_tokens = field::Empty,
        );
        let result = self
            .request(&endpoint, model, texts)
            .instrument(span.clone())
            .await;
        if let Ok((_, usage)) = &result {
            span.record("gen_ai.usage.input_tokens", usage.input_tokens);
        }
        result
    }
}
