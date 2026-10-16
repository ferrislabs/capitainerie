use capitainerie_common::{CoreError, OrganizationId, TranscriptId};

use super::Transcript;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait TranscriptRepository: Send {
    fn insert(
        &mut self,
        transcript: &Transcript,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: TranscriptId,
    ) -> impl Future<Output = Result<Option<Transcript>, CoreError>> + Send;
}
