use axum_extra::routing::TypedPath;
use capitainerie_core::{
    application::ModelListing,
    domain::provider::{
        ModelAssignments, ModelRef, ProviderKind, ProviderSummary, ports::CatalogModel,
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProviderKindDto {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "mistral")]
    Mistral,
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
}

impl From<ProviderKind> for ProviderKindDto {
    fn from(kind: ProviderKind) -> Self {
        match kind {
            ProviderKind::Anthropic => Self::Anthropic,
            ProviderKind::Mistral => Self::Mistral,
            ProviderKind::OpenAiCompatible => Self::OpenAiCompatible,
        }
    }
}

impl From<ProviderKindDto> for ProviderKind {
    fn from(kind: ProviderKindDto) -> Self {
        match kind {
            ProviderKindDto::Anthropic => Self::Anthropic,
            ProviderKindDto::Mistral => Self::Mistral,
            ProviderKindDto::OpenAiCompatible => Self::OpenAiCompatible,
        }
    }
}

impl std::fmt::Display for ProviderKindDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(ProviderKind::from(*self).as_str())
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/providers/{kind}")]
pub struct ProviderPath {
    pub kind: ProviderKindDto,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/providers/{kind}/test")]
pub struct TestProviderPath {
    pub kind: ProviderKindDto,
}

/// A configured provider. The key itself is never part of any response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderResponse {
    pub kind: ProviderKindDto,
    pub base_url: Option<String>,
    /// The last four characters of the key.
    pub preview: String,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
}

impl From<ProviderSummary> for ProviderResponse {
    fn from(summary: ProviderSummary) -> Self {
        Self {
            kind: summary.kind.into(),
            base_url: summary.base_url,
            preview: summary.preview,
            created_by: summary.created_by,
            updated_at: summary.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderListResponse {
    pub providers: Vec<ProviderResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CapabilitiesResponse {
    pub chat: bool,
    pub embeddings: bool,
}

/// Per million tokens, in millionths of the provider's currency unit.
#[derive(Debug, Serialize, ToSchema)]
pub struct PriceResponse {
    pub input_per_mtok_micros: Option<u64>,
    pub output_per_mtok_micros: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModelResponse {
    pub provider: ProviderKindDto,
    pub id: String,
    pub display_name: String,
    pub capabilities: CapabilitiesResponse,
    pub context_window: Option<u64>,
    pub price: Option<PriceResponse>,
}

impl From<CatalogModel> for ModelResponse {
    fn from(model: CatalogModel) -> Self {
        let priced = model.input_price_per_mtok_micros.is_some()
            || model.output_price_per_mtok_micros.is_some();
        Self {
            provider: model.provider.into(),
            id: model.id,
            display_name: model.display_name,
            capabilities: CapabilitiesResponse {
                chat: model.chat,
                embeddings: model.embeddings,
            },
            context_window: model.context_window,
            price: priced.then_some(PriceResponse {
                input_per_mtok_micros: model.input_price_per_mtok_micros,
                output_per_mtok_micros: model.output_price_per_mtok_micros,
            }),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderErrorResponse {
    pub provider: ProviderKindDto,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModelCatalogResponse {
    pub models: Vec<ModelResponse>,
    /// Providers that did not answer; the others are still listed.
    pub errors: Vec<ProviderErrorResponse>,
}

impl From<ModelListing> for ModelCatalogResponse {
    fn from(listing: ModelListing) -> Self {
        Self {
            models: listing
                .models
                .into_iter()
                .map(ModelResponse::from)
                .collect(),
            errors: listing
                .failures
                .into_iter()
                .map(|failure| ProviderErrorResponse {
                    provider: failure.provider.into(),
                    message: failure.message,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ModelRefDto {
    pub provider: ProviderKindDto,
    pub model: String,
}

impl From<ModelRef> for ModelRefDto {
    fn from(model: ModelRef) -> Self {
        Self {
            provider: model.provider.into(),
            model: model.model,
        }
    }
}

impl From<ModelRefDto> for ModelRef {
    fn from(model: ModelRefDto) -> Self {
        Self {
            provider: model.provider.into(),
            model: model.model,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ModelAssignmentsDto {
    pub extractor: ModelRefDto,
    pub judge: ModelRefDto,
    pub writer: ModelRefDto,
    pub embeddings: ModelRefDto,
}

impl From<ModelAssignments> for ModelAssignmentsDto {
    fn from(assignments: ModelAssignments) -> Self {
        Self {
            extractor: assignments.extractor.into(),
            judge: assignments.judge.into(),
            writer: assignments.writer.into(),
            embeddings: assignments.embeddings.into(),
        }
    }
}

impl From<ModelAssignmentsDto> for ModelAssignments {
    fn from(assignments: ModelAssignmentsDto) -> Self {
        Self {
            extractor: assignments.extractor.into(),
            judge: assignments.judge.into(),
            writer: assignments.writer.into(),
            embeddings: assignments.embeddings.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModelAssignmentsResponse {
    /// `null` while the organization has chosen nothing: runs then use the
    /// worker's default endpoint.
    pub assignments: Option<ModelAssignmentsDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_use_the_wire_names_of_the_domain() {
        for kind in ProviderKind::ALL {
            let dto = ProviderKindDto::from(kind);
            assert_eq!(
                serde_json::to_value(dto).unwrap(),
                serde_json::Value::String(kind.as_str().to_owned())
            );
            assert_eq!(ProviderKind::from(dto), kind);
        }
    }

    #[test]
    fn a_model_without_any_price_has_none() {
        let model = CatalogModel {
            provider: ProviderKind::Mistral,
            id: "m".into(),
            display_name: "M".into(),
            chat: true,
            embeddings: false,
            context_window: None,
            input_price_per_mtok_micros: None,
            output_price_per_mtok_micros: None,
        };
        assert!(ModelResponse::from(model).price.is_none());
    }
}
