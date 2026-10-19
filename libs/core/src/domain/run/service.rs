use capitainerie_common::{CoreError, RunId, TranscriptId};
use chrono::Utc;

use super::{Run, RunOverview, ports::RunRepository};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    event::ports::EventEmitter,
    provider::ModelAssignments,
    transcript::{
        MeetingMetadata, Transcript, TranscriptBody, TranscriptTitle, ports::TranscriptRepository,
    },
};

#[derive(Debug, Clone)]
pub struct SubmitTranscript {
    pub subject: Subject,
    pub title: String,
    pub body: String,
    pub metadata: MeetingMetadata,
}

pub struct RunService<'a, T, R, E, A> {
    transcripts: T,
    runs: R,
    emitter: &'a E,
    authorizer: &'a A,
}

impl<'a, T, R, E, A> RunService<'a, T, R, E, A>
where
    T: TranscriptRepository,
    R: RunRepository,
    E: EventEmitter,
    A: Authorizer,
{
    pub fn new(transcripts: T, runs: R, emitter: &'a E, authorizer: &'a A) -> Self {
        Self {
            transcripts,
            runs,
            emitter,
            authorizer,
        }
    }

    pub const ACTIVE_LIMIT: u32 = 100;

    /// Stores the transcript and queues the meeting pipeline on it.
    pub async fn submit(self, command: SubmitTranscript) -> Result<(Transcript, Run), CoreError> {
        self.submit_with_models(command, None).await
    }

    /// Like [`Self::submit`], freezing the organization's model choice into
    /// the run so a later change never affects it.
    pub async fn submit_with_models(
        mut self,
        command: SubmitTranscript,
        models: Option<ModelAssignments>,
    ) -> Result<(Transcript, Run), CoreError> {
        let organization_id = command.subject.organization_id;
        require(
            self.authorizer,
            AccessRequest::new(
                command.subject.clone(),
                Action::TranscriptCreate,
                ResourceKind::Transcript,
                None,
            ),
        )
        .await?;

        let now = Utc::now();
        let transcript = Transcript {
            id: TranscriptId::new(),
            organization_id,
            submitted_by: command.subject.id.clone(),
            title: TranscriptTitle::try_from(command.title)?,
            body: TranscriptBody::try_from(command.body)?,
            metadata: command.metadata,
            created_at: now,
        };
        let (mut run, queued) = Run::queue(
            organization_id,
            transcript.id,
            command.subject.id,
            Run::MEETING_PIPELINE.to_vec(),
            now,
        );

        run.models = models;
        self.transcripts.insert(&transcript).await?;
        self.runs.insert(&run).await?;
        self.emitter.emit(&queued)?;
        Ok((transcript, run))
    }

    pub async fn list(
        mut self,
        subject: Subject,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<RunOverview>, u64), CoreError> {
        let organization_id = subject.organization_id;
        self.require_read(subject, None).await?;
        self.runs
            .list(organization_id, limit.clamp(1, 100), offset)
            .await
    }

    pub async fn active(mut self, subject: Subject) -> Result<Vec<RunOverview>, CoreError> {
        let organization_id = subject.organization_id;
        self.require_read(subject, None).await?;
        self.runs
            .list_active(organization_id, Self::ACTIVE_LIMIT)
            .await
    }

    pub async fn get(mut self, subject: Subject, id: RunId) -> Result<RunOverview, CoreError> {
        let organization_id = subject.organization_id;
        self.require_read(subject, Some(id)).await?;
        self.runs
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    async fn require_read(&self, subject: Subject, id: Option<RunId>) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(
                subject,
                Action::RunRead,
                ResourceKind::Run,
                id.map(|id| id.0),
            )
            .in_run(id),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use capitainerie_common::OrganizationId;
    use capitainerie_events::{DomainEvent, run::RunQueued};

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        run::{RunStatus, RunSubject, ports::MockRunRepository},
        transcript::ports::MockTranscriptRepository,
    };

    #[derive(Default)]
    struct RecordingEmitter(Mutex<Vec<String>>);

    impl EventEmitter for RecordingEmitter {
        fn emit<E: DomainEvent>(&self, _event: &E) -> Result<(), CoreError> {
            self.0.lock().unwrap().push(E::KIND.to_owned());
            Ok(())
        }
    }

    fn subject() -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: OrganizationId::new(),
            actor: None,
            is_admin: false,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    fn command(title: &str) -> SubmitTranscript {
        SubmitTranscript {
            subject: subject(),
            title: title.into(),
            body: "Alice : je m'occupe du devis.".into(),
            metadata: MeetingMetadata::default(),
        }
    }

    #[tokio::test]
    async fn submit_stores_both_and_emits_run_queued() {
        let mut transcripts = MockTranscriptRepository::new();
        transcripts
            .expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let mut runs = MockRunRepository::new();
        runs.expect_insert()
            .times(1)
            .withf(|run| run.status == RunStatus::Queued)
            .returning(|_| Box::pin(async { Ok(()) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();

        let (transcript, run) = RunService::new(transcripts, runs, &emitter, &authorizer)
            .submit(command("Point hebdo"))
            .await
            .unwrap();

        assert_eq!(run.subject, RunSubject::Transcript(transcript.id));
        assert_eq!(run.initiated_by, "user-1");
        assert_eq!(*emitter.0.lock().unwrap(), [RunQueued::KIND]);
    }

    #[tokio::test]
    async fn a_denied_submission_writes_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "no".into(),
                })
            })
        });
        let emitter = RecordingEmitter::default();

        let error = RunService::new(
            MockTranscriptRepository::new(),
            MockRunRepository::new(),
            &emitter,
            &authorizer,
        )
        .submit(command("Point hebdo"))
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Forbidden { .. }));
        assert!(emitter.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_invalid_title_writes_nothing() {
        let emitter = RecordingEmitter::default();
        let authorizer = allow();

        let error = RunService::new(
            MockTranscriptRepository::new(),
            MockRunRepository::new(),
            &emitter,
            &authorizer,
        )
        .submit(command("   "))
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn get_hides_runs_of_other_organizations() {
        let mut runs = MockRunRepository::new();
        runs.expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();

        let error = RunService::new(MockTranscriptRepository::new(), runs, &emitter, &authorizer)
            .get(subject(), RunId::new())
            .await
            .unwrap_err();

        assert!(matches!(error, CoreError::NotFound));
    }
}
