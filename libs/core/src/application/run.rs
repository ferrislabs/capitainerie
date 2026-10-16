use capitainerie_common::{CoreError, RunId, TranscriptId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    authorization::Subject,
    run::{
        RunOverview,
        service::{RunService, SubmitTranscript},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmittedTranscript {
    pub transcript_id: TranscriptId,
    pub run_id: RunId,
}

impl CapitainerieUseCase {
    #[transactional(transcript, run, emitter, authz)]
    pub async fn submit_transcript(
        &self,
        command: SubmitTranscript,
    ) -> Result<SubmittedTranscript, CoreError> {
        let (transcript, run) =
            RunService::new(transcript_repository, run_repository, emitter, authz)
                .submit(command)
                .await?;
        Ok(SubmittedTranscript {
            transcript_id: transcript.id,
            run_id: run.id,
        })
    }

    #[transactional(transcript, run, emitter, authz)]
    pub async fn list_runs(
        &self,
        subject: Subject,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<RunOverview>, u64), CoreError> {
        RunService::new(transcript_repository, run_repository, emitter, authz)
            .list(subject, limit, offset)
            .await
    }

    #[transactional(transcript, run, emitter, authz)]
    pub async fn get_run(&self, subject: Subject, id: RunId) -> Result<RunOverview, CoreError> {
        RunService::new(transcript_repository, run_repository, emitter, authz)
            .get(subject, id)
            .await
    }
}
