use capitainerie_common::CoreError;
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::Subject,
        provider::{ModelTarget, service::ProviderService},
        transcription::{
            AudioClip, TranscribeError, Transcription,
            ports::{Transcriber, TranscriberFactory},
        },
    },
    infrastructure::provider::postgres::PgProviderRepository,
};

impl CapitainerieUseCase {
    pub async fn transcribe<F: TranscriberFactory>(
        &self,
        subject: Subject,
        audio: AudioClip,
        factory: &F,
    ) -> Result<Transcription, TranscribeError> {
        let target = self
            .transcriber_target(subject)
            .await?
            .ok_or(TranscribeError::NotConfigured)?;
        let ModelTarget {
            model,
            access: Some(access),
        } = target
        else {
            return Err(TranscribeError::NotConfigured);
        };
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
