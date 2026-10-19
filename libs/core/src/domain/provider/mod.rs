pub mod ports;
pub mod service;

use capitainerie_common::{CoreError, OrganizationId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{agent::SecretToken, execution::ExecutorModel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Anthropic,
    Mistral,
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Extractor,
    Judge,
    Writer,
    Embeddings,
    Transcriber,
    Reader,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider: ProviderKind,
    pub model: String,
}

/// Which model serves each agent role. Written into a run when it is queued,
/// so a configuration change never affects a run already waiting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAssignments {
    pub extractor: ModelRef,
    pub judge: ModelRef,
    pub writer: ModelRef,
    pub embeddings: ModelRef,
    #[serde(default)]
    pub transcriber: Option<ModelRef>,
    #[serde(default)]
    pub reader: Option<ModelRef>,
    /// Always an Anthropic model; runs queued before it existed take the default.
    #[serde(default)]
    pub executor: ExecutorModel,
}

/// An organization's key for one provider. The secret never leaves the
/// backend: the API only ever shows `preview`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCredential {
    pub organization_id: OrganizationId,
    pub kind: ProviderKind,
    pub secret: SecretToken,
    /// Required for `OpenAiCompatible`, ignored otherwise.
    pub base_url: Option<String>,
    pub preview: String,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
}

/// A configured provider as every member sees it: no secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSummary {
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub preview: String,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
}

/// What an adapter needs to call a provider on an organization's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAccess {
    pub kind: ProviderKind,
    pub secret: SecretToken,
    pub base_url: Option<String>,
}

/// The model of a role and how to reach it. `access` is `None` for the
/// worker's own default endpoint (environment configuration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTarget {
    pub model: ModelRef,
    pub access: Option<ProviderAccess>,
}

impl From<&ProviderCredential> for ProviderSummary {
    fn from(credential: &ProviderCredential) -> Self {
        Self {
            kind: credential.kind,
            base_url: credential.base_url.clone(),
            preview: credential.preview.clone(),
            created_by: credential.created_by.clone(),
            updated_at: credential.updated_at,
        }
    }
}

impl From<ProviderCredential> for ProviderAccess {
    fn from(credential: ProviderCredential) -> Self {
        Self {
            kind: credential.kind,
            secret: credential.secret,
            base_url: credential.base_url,
        }
    }
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 3] = [Self::Anthropic, Self::Mistral, Self::OpenAiCompatible];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Mistral => "mistral",
            Self::OpenAiCompatible => "openai_compatible",
        }
    }
}

impl TryFrom<&str> for ProviderKind {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown provider {value}")))
    }
}

impl AgentRole {
    pub const ALL: [AgentRole; 6] = [
        Self::Extractor,
        Self::Judge,
        Self::Writer,
        Self::Embeddings,
        Self::Transcriber,
        Self::Reader,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Extractor => "extractor",
            Self::Judge => "judge",
            Self::Writer => "writer",
            Self::Embeddings => "embeddings",
            Self::Transcriber => "transcriber",
            Self::Reader => "reader",
        }
    }
}

impl ModelAssignments {
    pub fn for_role(&self, role: AgentRole) -> Option<&ModelRef> {
        match role {
            AgentRole::Extractor => Some(&self.extractor),
            AgentRole::Judge => Some(&self.judge),
            AgentRole::Writer => Some(&self.writer),
            AgentRole::Embeddings => Some(&self.embeddings),
            AgentRole::Transcriber => self.transcriber.as_ref(),
            AgentRole::Reader => self.reader.as_ref(),
        }
    }

    pub fn require_role(&self, role: AgentRole) -> Result<&ModelRef, CoreError> {
        self.for_role(role)
            .ok_or_else(|| CoreError::Invalid(format!("no model is assigned to {}", role.as_str())))
    }

    pub fn check_transcriber(&self) -> Result<(), CoreError> {
        match &self.transcriber {
            Some(chosen) if chosen.provider == ProviderKind::Anthropic => Err(CoreError::Invalid(
                "anthropic offers no transcription model".into(),
            )),
            _ => Ok(()),
        }
    }

    pub fn check_reader(&self) -> Result<(), CoreError> {
        match &self.reader {
            Some(chosen) if chosen.provider != ProviderKind::Mistral => Err(CoreError::Invalid(
                "only mistral offers a document reader".into(),
            )),
            _ => Ok(()),
        }
    }

    /// Every role on the worker's default OpenAI-compatible endpoint.
    pub fn defaults(chat_model: &str, embedding_model: &str) -> Self {
        let chat = ModelRef {
            provider: ProviderKind::OpenAiCompatible,
            model: chat_model.to_owned(),
        };
        Self {
            extractor: chat.clone(),
            judge: chat.clone(),
            writer: chat,
            embeddings: ModelRef {
                provider: ProviderKind::OpenAiCompatible,
                model: embedding_model.to_owned(),
            },
            transcriber: None,
            reader: None,
            executor: ExecutorModel::default(),
        }
    }
}

/// `sk-…a1b2`: enough to recognise a key, never enough to use it.
pub fn preview(secret: &str) -> String {
    let tail: String = secret
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if secret.chars().count() <= 8 {
        return "…".to_owned();
    }
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_through_text() {
        for kind in ProviderKind::ALL {
            assert_eq!(ProviderKind::try_from(kind.as_str()).unwrap(), kind);
        }
        assert!(ProviderKind::try_from("openai").is_err());
    }

    #[test]
    fn kinds_serialize_as_their_text_form() {
        for kind in ProviderKind::ALL {
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                serde_json::Value::String(kind.as_str().to_owned())
            );
        }
    }

    #[test]
    fn assignments_stored_without_an_executor_take_the_default() {
        let mut stored = serde_json::to_value(ModelAssignments::defaults("chat", "embed")).unwrap();
        stored.as_object_mut().unwrap().remove("executor");
        let assignments: ModelAssignments = serde_json::from_value(stored).unwrap();
        assert_eq!(assignments.executor, ExecutorModel::default());
    }

    #[test]
    fn assignments_stored_before_the_transcriber_existed_still_load() {
        let mut stored = serde_json::to_value(ModelAssignments::defaults("chat", "embed")).unwrap();
        stored.as_object_mut().unwrap().remove("transcriber");
        let assignments: ModelAssignments = serde_json::from_value(stored).unwrap();
        assert_eq!(assignments.transcriber, None);
        assert_eq!(assignments.for_role(AgentRole::Transcriber), None);
        assert!(assignments.require_role(AgentRole::Transcriber).is_err());
        assert!(assignments.require_role(AgentRole::Judge).is_ok());
    }

    #[test]
    fn a_stored_transcriber_round_trips() {
        let mut assignments = ModelAssignments::defaults("chat", "embed");
        assignments.transcriber = Some(ModelRef {
            provider: ProviderKind::Mistral,
            model: "voxtral-mini-latest".into(),
        });
        let stored = serde_json::to_value(&assignments).unwrap();
        assert_eq!(stored["transcriber"]["model"], "voxtral-mini-latest");
        assert_eq!(
            serde_json::from_value::<ModelAssignments>(stored).unwrap(),
            assignments
        );
    }

    fn reader_assignment(provider: ProviderKind) -> ModelAssignments {
        let mut assignments = ModelAssignments::defaults("chat", "embed");
        assignments.reader = Some(ModelRef {
            provider,
            model: "mistral-ocr-latest".into(),
        });
        assignments
    }

    #[test]
    fn assignments_stored_before_the_reader_existed_still_load() {
        let mut stored = serde_json::to_value(ModelAssignments::defaults("chat", "embed")).unwrap();
        stored.as_object_mut().unwrap().remove("reader");
        let assignments: ModelAssignments = serde_json::from_value(stored).unwrap();
        assert_eq!(assignments.reader, None);
        assert_eq!(assignments.for_role(AgentRole::Reader), None);
        assert!(assignments.require_role(AgentRole::Reader).is_err());
        assert_eq!(AgentRole::Reader.as_str(), "reader");
    }

    #[test]
    fn a_stored_reader_round_trips() {
        let assignments = reader_assignment(ProviderKind::Mistral);
        let stored = serde_json::to_value(&assignments).unwrap();
        assert_eq!(stored["reader"]["model"], "mistral-ocr-latest");
        assert_eq!(
            serde_json::from_value::<ModelAssignments>(stored).unwrap(),
            assignments
        );
    }

    #[test]
    fn only_mistral_may_read_documents() {
        reader_assignment(ProviderKind::Mistral)
            .check_reader()
            .unwrap();
        ModelAssignments::defaults("chat", "embed")
            .check_reader()
            .unwrap();
        for provider in [ProviderKind::Anthropic, ProviderKind::OpenAiCompatible] {
            assert!(matches!(
                reader_assignment(provider).check_reader(),
                Err(CoreError::Invalid(message)) if message.contains("only mistral")
            ));
        }
    }

    #[test]
    fn a_preview_shows_only_the_last_four_characters() {
        assert_eq!(preview("sk-ant-api03-abcdefghij1234"), "…1234");
        assert_eq!(preview("short"), "…");
    }

    #[test]
    fn defaults_put_every_role_on_the_local_endpoint() {
        let defaults = ModelAssignments::defaults("qwen2.5:7b-instruct", "bge-m3");
        assert_eq!(
            defaults.require_role(AgentRole::Writer).unwrap().model,
            "qwen2.5:7b-instruct"
        );
        assert_eq!(
            defaults.require_role(AgentRole::Embeddings).unwrap().model,
            "bge-m3"
        );
        assert!(
            AgentRole::ALL
                .iter()
                .filter(|role| !matches!(role, AgentRole::Transcriber | AgentRole::Reader))
                .all(|role| matches!(
                    defaults.for_role(*role),
                    Some(model) if model.provider == ProviderKind::OpenAiCompatible
                ))
        );
        assert_eq!(defaults.for_role(AgentRole::Transcriber), None);
        assert_eq!(defaults.for_role(AgentRole::Reader), None);
    }
}
