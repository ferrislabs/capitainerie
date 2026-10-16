use capitainerie_common::CoreError;

use super::Transcript;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait TranscriptRepository: Send {
    fn insert(
        &mut self,
        transcript: &Transcript,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
