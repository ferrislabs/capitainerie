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
    /// Whether the model can transcribe audio.
    pub transcription: bool,
    /// Whether the model can read a PDF (Mistral OCR).
    pub ocr: bool,
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
                ocr: model.ocr,
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
    /// The model that transcribes audio (mistral or openai_compatible only).
    /// On a PUT, an absent key keeps the current transcriber and an explicit
    /// `null` clears it.
    #[serde(default, deserialize_with = "present_or_null")]
    #[schema(value_type = Option<ModelRefDto>)]
    pub transcriber: Option<Option<ModelRefDto>>,
    /// The model that reads PDFs (mistral only). On a PUT, an absent key keeps
    /// the current reader and an explicit `null` clears it.
    #[serde(default, deserialize_with = "present_or_null")]
    #[schema(value_type = Option<ModelRefDto>)]
    pub reader: Option<Option<ModelRefDto>>,
    /// The Anthropic model Claude Code runs on. When absent on a PUT, the
    /// current value (or the default) is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
}

fn present_or_null<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

impl From<ModelAssignments> for ModelAssignmentsDto {
    fn from(assignments: ModelAssignments) -> Self {
        Self {
            extractor: assignments.extractor.into(),
            judge: assignments.judge.into(),
            writer: assignments.writer.into(),
            embeddings: assignments.embeddings.into(),
            transcriber: Some(assignments.transcriber.map(Into::into)),
            reader: Some(assignments.reader.map(Into::into)),
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
        let transcriber = match self.transcriber {
            Some(chosen) => chosen.map(Into::into),
            None => current.and_then(|c| c.transcriber.clone()),
        };
        let reader = match self.reader {
            Some(chosen) => chosen.map(Into::into),
            None => current.and_then(|c| c.reader.clone()),
        };
        Ok(ModelAssignments {
            extractor: self.extractor.into(),
            judge: self.judge.into(),
            writer: self.writer.into(),
            embeddings: self.embeddings.into(),
            transcriber,
            reader,
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

    fn wire(transcriber: Option<serde_json::Value>) -> ModelAssignmentsDto {
        let mut body = serde_json::json!({
            "extractor": {"provider": "mistral", "model": "m"},
            "judge": {"provider": "mistral", "model": "m"},
            "writer": {"provider": "mistral", "model": "m"},
            "embeddings": {"provider": "mistral", "model": "e"},
        });
        if let Some(value) = transcriber {
            body["transcriber"] = value;
        }
        serde_json::from_value(body).unwrap()
    }

    #[test]
    fn an_absent_transcriber_keeps_the_current_one_and_null_clears_it() {
        let mut current = ModelAssignments::defaults("chat", "embed");
        current.transcriber = Some(ModelRef {
            provider: ProviderKind::Mistral,
            model: "voxtral-mini-latest".into(),
        });

        let kept = wire(None).into_assignments(Some(&current)).unwrap();
        let cleared = wire(Some(serde_json::Value::Null))
            .into_assignments(Some(&current))
            .unwrap();
        let replaced = wire(Some(
            serde_json::json!({"provider": "openai_compatible", "model": "whisper-1"}),
        ))
        .into_assignments(Some(&current))
        .unwrap();
        let first_time = wire(None).into_assignments(None).unwrap();

        assert_eq!(kept.transcriber, current.transcriber);
        assert_eq!(cleared.transcriber, None);
        assert_eq!(
            replaced.transcriber.map(|m| (m.provider, m.model)),
            Some((ProviderKind::OpenAiCompatible, "whisper-1".to_owned()))
        );
        assert_eq!(first_time.transcriber, None);
    }

    #[test]
    fn a_stored_transcriber_is_serialized_and_none_is_an_explicit_null() {
        let mut assignments = ModelAssignments::defaults("chat", "embed");
        let none = serde_json::to_value(ModelAssignmentsDto::from(assignments.clone())).unwrap();
        assignments.transcriber = Some(ModelRef {
            provider: ProviderKind::Mistral,
            model: "voxtral-mini-latest".into(),
        });
        let some = serde_json::to_value(ModelAssignmentsDto::from(assignments)).unwrap();

        assert_eq!(none["transcriber"], serde_json::Value::Null);
        assert_eq!(some["transcriber"]["model"], "voxtral-mini-latest");
    }

    fn wire_reader(reader: Option<serde_json::Value>) -> ModelAssignmentsDto {
        let mut body = serde_json::to_value(wire(None)).unwrap();
        body.as_object_mut().unwrap().remove("reader");
        if let Some(value) = reader {
            body["reader"] = value;
        }
        serde_json::from_value(body).unwrap()
    }

    #[test]
    fn an_absent_reader_keeps_the_current_one_and_null_clears_it() {
        let mut current = ModelAssignments::defaults("chat", "embed");
        current.reader = Some(ModelRef {
            provider: ProviderKind::Mistral,
            model: "mistral-ocr-latest".into(),
        });

        let kept = wire_reader(None).into_assignments(Some(&current)).unwrap();
        let cleared = wire_reader(Some(serde_json::Value::Null))
            .into_assignments(Some(&current))
            .unwrap();
        let replaced = wire_reader(Some(
            serde_json::json!({"provider": "mistral", "model": "mistral-ocr-2505"}),
        ))
        .into_assignments(Some(&current))
        .unwrap();
        let first_time = wire_reader(None).into_assignments(None).unwrap();

        assert_eq!(kept.reader, current.reader);
        assert_eq!(cleared.reader, None);
        assert_eq!(
            replaced.reader.map(|m| (m.provider, m.model)),
            Some((ProviderKind::Mistral, "mistral-ocr-2505".to_owned()))
        );
        assert_eq!(first_time.reader, None);
    }

    #[test]
    fn a_stored_reader_is_serialized_and_none_is_an_explicit_null() {
        let mut assignments = ModelAssignments::defaults("chat", "embed");
        let none = serde_json::to_value(ModelAssignmentsDto::from(assignments.clone())).unwrap();
        assignments.reader = Some(ModelRef {
            provider: ProviderKind::Mistral,
            model: "mistral-ocr-latest".into(),
        });
        let some = serde_json::to_value(ModelAssignmentsDto::from(assignments)).unwrap();

        assert_eq!(none["reader"], serde_json::Value::Null);
        assert_eq!(some["reader"]["model"], "mistral-ocr-latest");
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
            ocr: false,
            context_window: None,
            input_price_per_mtok_micros: None,
            output_price_per_mtok_micros: None,
        };
        assert!(ModelResponse::from(model).price.is_none());
    }
}
