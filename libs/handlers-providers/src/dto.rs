use axum_extra::routing::TypedPath;
use capitainerie_common::CoreError;
use capitainerie_core::{
    application::ModelListing,
    domain::execution::ExecutorModel,
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
    pub transcription: bool,
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
                transcription: model.transcription,
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
    #[serde(default)]
    pub transcriber: Option<ModelRefDto>,
    /// The Anthropic model Claude Code runs on. When absent on a PUT, the
    /// current value (or the default) is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
}

impl From<ModelAssignments> for ModelAssignmentsDto {
    fn from(assignments: ModelAssignments) -> Self {
        Self {
            extractor: assignments.extractor.into(),
            judge: assignments.judge.into(),
            writer: assignments.writer.into(),
            embeddings: assignments.embeddings.into(),
            transcriber: assignments.transcriber.map(Into::into),
            executor: Some(assignments.executor.as_str().to_owned()),
        }
    }
}

impl ModelAssignmentsDto {
    pub fn into_assignments(
        self,
        current: Option<&ModelAssignments>,
    ) -> Result<ModelAssignments, CoreError> {
        let executor = match self.executor {
            Some(model) => ExecutorModel::new(model)?,
            None => current.map(|c| c.executor.clone()).unwrap_or_default(),
        };
        Ok(ModelAssignments {
            extractor: self.extractor.into(),
            judge: self.judge.into(),
            writer: self.writer.into(),
            embeddings: self.embeddings.into(),
            transcriber: self.transcriber.map(Into::into),
            executor,
        })
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

    fn dto(executor: Option<&str>) -> ModelAssignmentsDto {
        let defaults = ModelAssignmentsDto::from(ModelAssignments::defaults("chat", "embed"));
        ModelAssignmentsDto {
            executor: executor.map(str::to_owned),
            ..defaults
        }
    }

    #[test]
    fn an_absent_executor_keeps_the_current_one_or_the_default() {
        let mut current = ModelAssignments::defaults("chat", "embed");
        current.executor = ExecutorModel::new("claude-opus-5").unwrap();

        let kept = dto(None).into_assignments(Some(&current)).unwrap();
        let defaulted = dto(None).into_assignments(None).unwrap();
        let replaced = dto(Some("claude-haiku-5-5"))
            .into_assignments(Some(&current))
            .unwrap();

        assert_eq!(kept.executor.as_str(), "claude-opus-5");
        assert_eq!(defaulted.executor, ExecutorModel::default());
        assert_eq!(replaced.executor.as_str(), "claude-haiku-5-5");
        assert!(dto(Some(" ")).into_assignments(None).is_err());
    }

    #[test]
    fn the_transcriber_is_optional_on_the_wire_and_survives_the_conversion() {
        let without: ModelAssignmentsDto = serde_json::from_value(serde_json::json!({
            "extractor": {"provider": "mistral", "model": "m"},
            "judge": {"provider": "mistral", "model": "m"},
            "writer": {"provider": "mistral", "model": "m"},
            "embeddings": {"provider": "mistral", "model": "e"},
        }))
        .unwrap();
        assert_eq!(without.transcriber, None);
        assert_eq!(without.into_assignments(None).unwrap().transcriber, None);

        let mut with = dto(None);
        with.transcriber = Some(ModelRefDto {
            provider: ProviderKindDto::Mistral,
            model: "voxtral-mini-latest".into(),
        });
        let assignments = with.clone().into_assignments(None).unwrap();
        assert_eq!(
            assignments.transcriber.as_ref().map(|m| m.model.as_str()),
            Some("voxtral-mini-latest")
        );
        assert_eq!(
            ModelAssignmentsDto::from(assignments).transcriber,
            with.transcriber
        );
    }

    #[test]
    fn a_model_without_any_price_has_none() {
        let model = CatalogModel {
            provider: ProviderKind::Mistral,
            id: "m".into(),
            display_name: "M".into(),
            chat: true,
            embeddings: false,
            transcription: false,
            context_window: None,
            input_price_per_mtok_micros: None,
            output_price_per_mtok_micros: None,
        };
        assert!(ModelResponse::from(model).price.is_none());
    }
}
