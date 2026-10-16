use std::{sync::Arc, time::Duration};

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::Usage,
    similarity::{Embedding, ports::Embedder},
};
use rig_reqwest::reqwest;
use serde::Deserialize;
use serde_json::json;
use tracing::{Instrument, field, info_span};

use crate::{
    args::{EmbeddingArgs, LlmArgs},
    error::RuntimeError,
};

/// Embeds text through an OpenAI-compatible `/embeddings` endpoint, the same
/// server as the chat model.
#[derive(Clone)]
pub struct OpenAiEmbedder {
    http: reqwest::Client,
    url: Arc<str>,
    api_key: Arc<str>,
    model: Arc<str>,
    dimensions: usize,
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

impl OpenAiEmbedder {
    pub fn new(llm: &LlmArgs, embedding: &EmbeddingArgs) -> Result<Self, RuntimeError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(llm.timeout_secs))
            .build()
            .map_err(|error| RuntimeError::HttpClient(error.to_string()))?;
        Ok(Self {
            http,
            url: Arc::from(format!("{}/embeddings", llm.base_url.trim_end_matches('/'))),
            api_key: Arc::from(llm.api_key.as_str()),
            model: Arc::from(embedding.model.as_str()),
            dimensions: embedding.dimensions,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    async fn request(&self, texts: &[String]) -> Result<(Vec<Embedding>, Usage), CoreError> {
        let response = self
            .http
            .post(&*self.url)
            .bearer_auth(&*self.api_key)
            .json(&json!({ "model": &*self.model, "input": texts }))
            .send()
            .await
            .map_err(|error| CoreError::Internal(format!("the embedding call failed: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(CoreError::Internal(format!(
                "the embedding call returned {status}: {}",
                crate::usage::truncate_chars(&body, 300)
            )));
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
        let mut embeddings = Vec::with_capacity(items.len());
        for item in items {
            if item.embedding.len() != self.dimensions {
                return Err(CoreError::Internal(format!(
                    "model {} returned {} dimensions, EMBEDDING_DIMENSIONS is {}",
                    self.model,
                    item.embedding.len(),
                    self.dimensions
                )));
            }
            embeddings.push(Embedding(item.embedding));
        }
        Ok((
            embeddings,
            Usage {
                model: self.model.to_string(),
                input_tokens: tokens,
                output_tokens: 0,
            },
        ))
    }
}

impl Embedder for OpenAiEmbedder {
    async fn embed(&self, texts: &[String]) -> Result<(Vec<Embedding>, Usage), CoreError> {
        if texts.is_empty() {
            return Ok((
                Vec::new(),
                Usage {
                    model: self.model.to_string(),
                    ..Usage::default()
                },
            ));
        }
        let span = info_span!(
            "embeddings",
            otel.name = format!("embeddings {}", self.model),
            gen_ai.operation.name = "embeddings",
            gen_ai.request.model = &*self.model,
            gen_ai.usage.input_tokens = field::Empty,
        );
        let result = self.request(texts).instrument(span.clone()).await;
        if let Ok((_, usage)) = &result {
            span.record("gen_ai.usage.input_tokens", usage.input_tokens);
        }
        result
    }
}
