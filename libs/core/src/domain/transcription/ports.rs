use capitainerie_common::CoreError;

use super::{AudioClip, TranscriptionText};
use crate::domain::provider::{ModelRef, ProviderAccess};

pub trait Transcriber: Send + Sync {
    fn transcribe(
        &self,
        model: &ModelRef,
        audio: AudioClip,
    ) -> impl Future<Output = Result<TranscriptionText, CoreError>> + Send;
}

pub trait TranscriberFactory: Send + Sync {
    type Bound: Transcriber;

    fn bind(&self, access: &ProviderAccess) -> Self::Bound;
}
