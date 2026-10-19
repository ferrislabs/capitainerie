use capitainerie_common::CoreError;
use capitainerie_core::domain::provider::{
    ProviderAccess, ProviderKind,
    ports::{CatalogModel, ModelCatalog},
};
use rig_reqwest::reqwest;
use serde_json::Value;

use crate::{
    client::{ANTHROPIC_BASE, MISTRAL_BASE, http_client},
    error::RuntimeError,
    prices::price_of,
    usage::truncate_chars,
};

const PAGE_LIMIT: u32 = 1000;
const MAX_PAGES: usize = 10;
const TIMEOUT_SECS: u64 = 15;
const ERROR_MAX_CHARS: usize = 200;
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Clone)]
pub struct RigModelCatalog {
    http: reqwest::Client,
}

impl RigModelCatalog {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            http: http_client(TIMEOUT_SECS)?,
        })
    }

    async fn get(&self, request: reqwest::RequestBuilder) -> Result<Value, CoreError> {
        let response = request.send().await.map_err(|error| {
            CoreError::Internal(format!("the provider is unreachable: {error}"))
        })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            CoreError::Internal(format!("the model list is unreadable: {error}"))
        })?;
        if !status.is_success() {
            let detail = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|value| {
                    value["error"]["message"]
                        .as_str()
                        .or_else(|| value["message"].as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| truncate_chars(&body, ERROR_MAX_CHARS));
            let hint = match status.as_u16() {
                401 | 403 => " (check the API key)",
                _ => "",
            };
            return Err(CoreError::Internal(format!(
                "the provider answered {status}{hint}: {}",
                truncate_chars(&detail, ERROR_MAX_CHARS)
            )));
        }
        serde_json::from_str(&body)
            .map_err(|error| CoreError::Internal(format!("the model list is unreadable: {error}")))
    }

    async fn anthropic(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        let base = base_of(access, ANTHROPIC_BASE);
        let mut models = Vec::new();
        let mut after: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut url = format!("{base}/v1/models?limit={PAGE_LIMIT}");
            if let Some(after) = &after {
                url.push_str("&after_id=");
                url.push_str(after);
            }
            let request = self
                .http
                .get(url)
                .header("x-api-key", access.secret.expose())
                .header("anthropic-version", ANTHROPIC_VERSION);
            let page = self.get(request).await?;
            models.extend(anthropic_models(&page));
            match (page["has_more"].as_bool(), page["last_id"].as_str()) {
                (Some(true), Some(last)) => after = Some(last.to_owned()),
                _ => break,
            }
        }
        Ok(models)
    }

    async fn mistral(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        let base = base_of(access, MISTRAL_BASE);
        let page = self
            .get(
                self.http
                    .get(format!("{base}/v1/models"))
                    .bearer_auth(access.secret.expose()),
            )
            .await?;
        Ok(mistral_models(&page))
    }

    async fn openai_compatible(
        &self,
        access: &ProviderAccess,
    ) -> Result<Vec<CatalogModel>, CoreError> {
        let base = access
            .base_url
            .as_deref()
            .map(|url| url.trim_end_matches('/'))
            .ok_or_else(|| {
                CoreError::Invalid("the OpenAI-compatible provider has no base url".into())
            })?;
        let mut request = self.http.get(format!("{base}/models"));
        if !access.secret.expose().is_empty() {
            request = request.bearer_auth(access.secret.expose());
        }
        let page = self.get(request).await?;
        Ok(openai_models(&page))
    }
}

fn base_of<'a>(access: &'a ProviderAccess, default: &'a str) -> &'a str {
    access
        .base_url
        .as_deref()
        .map_or(default, |url| url.trim_end_matches('/'))
}

fn model(
    provider: ProviderKind,
    id: &str,
    display_name: Option<&str>,
    embeddings: bool,
    chat: bool,
    context_window: Option<u64>,
) -> CatalogModel {
    let price = price_of(provider, id);
    CatalogModel {
        provider,
        id: id.to_owned(),
        display_name: display_name.unwrap_or(id).to_owned(),
        chat,
        embeddings,
        transcription: provider != ProviderKind::Anthropic && is_transcription_model(id),
        context_window,
        input_price_per_mtok_micros: price.map(|price| price.input_per_mtok_micros),
        output_price_per_mtok_micros: price.map(|price| price.output_per_mtok_micros),
    }
}

fn is_transcription_model(id: &str) -> bool {
    let id = id.to_lowercase();
    id.contains("voxtral") || id.contains("whisper")
}

fn sorted(mut models: Vec<CatalogModel>) -> Vec<CatalogModel> {
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    models
}

fn anthropic_models(page: &Value) -> Vec<CatalogModel> {
    page["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let id = entry["id"].as_str()?;
            Some(model(
                ProviderKind::Anthropic,
                id,
                entry["display_name"].as_str(),
                false,
                true,
                entry["max_input_tokens"].as_u64(),
            ))
        })
        .collect()
}

fn mistral_models(page: &Value) -> Vec<CatalogModel> {
    sorted(
        page["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let id = entry["id"].as_str()?;
                let embeddings = id.contains("embed");
                let chat = !embeddings
                    && entry["capabilities"]["completion_chat"]
                        .as_bool()
                        .unwrap_or(false);
                Some(model(
                    ProviderKind::Mistral,
                    id,
                    entry["name"].as_str(),
                    embeddings,
                    chat,
                    entry["max_context_length"].as_u64(),
                ))
            })
            .collect(),
    )
}

fn is_embedding_model(id: &str) -> bool {
    let id = id.to_lowercase();
    ["embed", "bge-", "nomic", "e5-", "gte-", "minilm"]
        .iter()
        .any(|marker| id.contains(marker))
}

fn openai_models(page: &Value) -> Vec<CatalogModel> {
    sorted(
        page["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let id = entry["id"].as_str()?;
                let embeddings = is_embedding_model(id);
                Some(model(
                    ProviderKind::OpenAiCompatible,
                    id,
                    None,
                    embeddings,
                    !embeddings,
                    None,
                ))
            })
            .collect(),
    )
}

impl ModelCatalog for RigModelCatalog {
    async fn list_models(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        match access.kind {
            ProviderKind::Anthropic => self.anthropic(access).await.map(sorted),
            ProviderKind::Mistral => self.mistral(access).await,
            ProviderKind::OpenAiCompatible => self.openai_compatible(access).await,
        }
    }
}
