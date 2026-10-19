use capitainerie_common::CoreError;
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::Subject,
        provider::{ModelRef, ModelTarget, ProviderAccess, ProviderKind, service::ProviderService},
        transcription::{
            AudioClip, TranscribeError, Transcription,
            ports::{Transcriber, TranscriberFactory},
        },
    },
    infrastructure::provider::postgres::PgProviderRepository,
};

#[derive(Debug, Clone)]
pub struct PreparedTranscription {
    model: ModelRef,
    access: ProviderAccess,
}

impl CapitainerieUseCase {
    pub async fn prepare_transcription(
        &self,
        subject: Subject,
    ) -> Result<PreparedTranscription, TranscribeError> {
        let target = self
            .transcriber_target(subject)
            .await?
            .ok_or(TranscribeError::NotConfigured)?;
        match target {
            ModelTarget {
                model,
                access: Some(access),
            } if usable(&access) => Ok(PreparedTranscription { model, access }),
            _ => Err(TranscribeError::NotConfigured),
        }
    }

    pub async fn transcribe<F: TranscriberFactory>(
        &self,
        prepared: PreparedTranscription,
        audio: AudioClip,
        factory: &F,
    ) -> Result<Transcription, TranscribeError> {
        let PreparedTranscription { model, access } = prepared;
        tracing::info!(
            bytes = audio.bytes.len(),
            provider = model.provider.as_str(),
            model = model.model.as_str(),
            "transcribing audio"
        );
        let text = factory
            .bind(&access)
            .transcribe(&model, audio)
            .await
            .map_err(|error| match error {
                CoreError::Internal(message) | CoreError::Invalid(message) => {
                    TranscribeError::Provider(message)
                }
                other => TranscribeError::Core(other),
            })?;
        Ok(Transcription {
            text: text.0,
            model,
        })
    }

    #[transactional(model_assignment, authz)]
    async fn transcriber_target(&self, subject: Subject) -> Result<Option<ModelTarget>, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .transcriber_target(subject)
            .await
    }
}

fn usable(access: &ProviderAccess) -> bool {
    match access.kind {
        ProviderKind::Anthropic => false,
        ProviderKind::Mistral => true,
        ProviderKind::OpenAiCompatible => access
            .base_url
            .as_deref()
            .is_some_and(|url| !url.trim().is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent::SecretToken;

    fn access(kind: ProviderKind, base_url: Option<&str>) -> ProviderAccess {
        ProviderAccess {
            kind,
            secret: SecretToken::new("k"),
            base_url: base_url.map(str::to_owned),
        }
    }

    #[test]
    fn only_a_reachable_non_anthropic_provider_can_transcribe() {
        assert!(usable(&access(ProviderKind::Mistral, None)));
        assert!(usable(&access(
            ProviderKind::OpenAiCompatible,
            Some("http://localhost:11434/v1")
        )));
        assert!(!usable(&access(ProviderKind::OpenAiCompatible, None)));
        assert!(!usable(&access(ProviderKind::OpenAiCompatible, Some("  "))));
        assert!(!usable(&access(ProviderKind::Anthropic, None)));
    }
}
