pub mod locate;
pub mod ports;
pub mod service;
pub mod step;

use capitainerie_common::{CardId, CoreError, OrganizationId, RunId, TranscriptId};
use capitainerie_events::run::RunQueued;
use chrono::{DateTime, Utc};

use crate::domain::execution::RepositoryRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Extract,
    Deduplicate,
    Write,
    Execute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunSubject {
    Transcript(TranscriptId),
    Card {
        card_id: CardId,
        repository: RepositoryRef,
        base_branch: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: RunId,
    pub organization_id: OrganizationId,
    pub subject: RunSubject,
    pub initiated_by: String,
    pub status: RunStatus,
    pub pipeline: Vec<StepKind>,
    pub failure: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Trace id of the worker span that runs it, to open the trace backend.
    pub trace_id: Option<String>,
    /// Models frozen at submission; `None` for runs queued before model
    /// assignments existed, which use the worker's defaults.
    pub models: Option<crate::domain::provider::ModelAssignments>,
}

/// A run as the console lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOverview {
    pub run: Run,
    pub title: Option<String>,
}

impl Run {
    pub const MEETING_PIPELINE: [StepKind; 3] =
        [StepKind::Extract, StepKind::Deduplicate, StepKind::Write];

    pub fn queue(
        organization_id: OrganizationId,
        transcript_id: TranscriptId,
        initiated_by: String,
        pipeline: Vec<StepKind>,
        now: DateTime<Utc>,
    ) -> (Self, RunQueued) {
        Self::queued(
            organization_id,
            RunSubject::Transcript(transcript_id),
            initiated_by,
            pipeline,
            now,
        )
    }

    pub const EXECUTION_PIPELINE: [StepKind; 1] = [StepKind::Execute];

    pub fn queue_execution(
        organization_id: OrganizationId,
        card_id: CardId,
        repository: RepositoryRef,
        base_branch: String,
        initiated_by: String,
        now: DateTime<Utc>,
    ) -> (Self, RunQueued) {
        Self::queued(
            organization_id,
            RunSubject::Card {
                card_id,
                repository,
                base_branch,
            },
            initiated_by,
            Self::EXECUTION_PIPELINE.to_vec(),
            now,
        )
    }

    fn queued(
        organization_id: OrganizationId,
        subject: RunSubject,
        initiated_by: String,
        pipeline: Vec<StepKind>,
        now: DateTime<Utc>,
    ) -> (Self, RunQueued) {
        let run = Self {
            id: RunId::new(),
            organization_id,
            subject,
            initiated_by,
            status: RunStatus::Queued,
            pipeline,
            failure: None,
            created_at: now,
            started_at: None,
            finished_at: None,
            trace_id: None,
            models: None,
        };
        let event = RunQueued {
            run_id: run.id.0,
            organization_id: organization_id.0,
            transcript_id: run.subject.transcript_id().map(|id| id.0),
            card_id: run.subject.card_id().map(|id| id.0),
            initiated_by: run.initiated_by.clone(),
            pipeline: run
                .pipeline
                .iter()
                .map(|step| step.as_str().to_owned())
                .collect(),
        };
        (run, event)
    }

    pub fn start(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        self.transition(&[RunStatus::Queued], RunStatus::Running)?;
        self.started_at = Some(now);
        Ok(())
    }

    pub fn succeed(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        self.transition(&[RunStatus::Running], RunStatus::Succeeded)?;
        self.finished_at = Some(now);
        Ok(())
    }

    pub fn fail(&mut self, reason: String, now: DateTime<Utc>) -> Result<(), CoreError> {
        self.transition(&[RunStatus::Queued, RunStatus::Running], RunStatus::Failed)?;
        self.failure = Some(reason);
        self.finished_at = Some(now);
        Ok(())
    }

    pub fn cancel(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        self.transition(
            &[RunStatus::Queued, RunStatus::Running],
            RunStatus::Cancelled,
        )?;
        self.finished_at = Some(now);
        Ok(())
    }

    fn transition(&mut self, from: &[RunStatus], to: RunStatus) -> Result<(), CoreError> {
        if !from.contains(&self.status) {
            return Err(CoreError::Conflict(format!(
                "run {} cannot go from {} to {}",
                self.id,
                self.status.as_str(),
                to.as_str()
            )));
        }
        self.status = to;
        Ok(())
    }
}

impl RunSubject {
    pub fn transcript_id(&self) -> Option<TranscriptId> {
        match self {
            Self::Transcript(id) => Some(*id),
            Self::Card { .. } => None,
        }
    }

    pub fn card_id(&self) -> Option<CardId> {
        match self {
            Self::Transcript(_) => None,
            Self::Card { card_id, .. } => Some(*card_id),
        }
    }
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl TryFrom<&str> for RunStatus {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(CoreError::Internal(format!("unknown run status {other}"))),
        }
    }
}

impl StepKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::Deduplicate => "deduplicate",
            Self::Write => "write",
            Self::Execute => "execute",
        }
    }
}

impl TryFrom<&str> for StepKind {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "extract" => Ok(Self::Extract),
            "deduplicate" => Ok(Self::Deduplicate),
            "write" => Ok(Self::Write),
            "execute" => Ok(Self::Execute),
            other => Err(CoreError::Internal(format!("unknown step kind {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued() -> Run {
        Run::queue(
            OrganizationId::new(),
            TranscriptId::new(),
            "user-1".into(),
            Run::MEETING_PIPELINE.to_vec(),
            Utc::now(),
        )
        .0
    }

    #[test]
    fn queue_emits_the_matching_event() {
        let (run, event) = Run::queue(
            OrganizationId::new(),
            TranscriptId::new(),
            "user-1".into(),
            Run::MEETING_PIPELINE.to_vec(),
            Utc::now(),
        );
        assert_eq!(run.status, RunStatus::Queued);
        assert_eq!(event.run_id, run.id.0);
        assert_eq!(
            event.transcript_id,
            run.subject.transcript_id().map(|id| id.0)
        );
        assert_eq!(event.card_id, None);
        assert_eq!(event.pipeline, ["extract", "deduplicate", "write"]);
    }

    #[test]
    fn happy_path_stamps_start_and_finish() {
        let mut run = queued();
        run.start(Utc::now()).unwrap();
        run.succeed(Utc::now()).unwrap();
        assert_eq!(run.status, RunStatus::Succeeded);
        assert!(run.started_at.is_some() && run.finished_at.is_some());
    }

    #[test]
    fn a_finished_run_is_final() {
        let mut run = queued();
        run.cancel(Utc::now()).unwrap();
        assert!(matches!(run.start(Utc::now()), Err(CoreError::Conflict(_))));
        assert!(run.fail("late".into(), Utc::now()).is_err());
        assert_eq!(run.status, RunStatus::Cancelled);
    }

    #[test]
    fn a_queued_run_cannot_succeed_without_running() {
        let mut run = queued();
        assert!(run.succeed(Utc::now()).is_err());
        assert_eq!(run.status, RunStatus::Queued);
    }

    #[test]
    fn statuses_and_steps_round_trip_through_text() {
        for status in [
            RunStatus::Queued,
            RunStatus::Running,
            RunStatus::Succeeded,
            RunStatus::Failed,
            RunStatus::Cancelled,
        ] {
            assert_eq!(RunStatus::try_from(status.as_str()).unwrap(), status);
        }
        for step in Run::MEETING_PIPELINE
            .into_iter()
            .chain(Run::EXECUTION_PIPELINE)
        {
            assert_eq!(StepKind::try_from(step.as_str()).unwrap(), step);
        }
    }

    #[test]
    fn queue_execution_binds_the_card_and_the_repository() {
        let card_id = CardId::new();
        let (run, event) = Run::queue_execution(
            OrganizationId::new(),
            card_id,
            "acme/web".parse().unwrap(),
            "main".into(),
            "user-1".into(),
            Utc::now(),
        );
        assert_eq!(run.status, RunStatus::Queued);
        assert_eq!(run.pipeline, [StepKind::Execute]);
        assert_eq!(run.subject.card_id(), Some(card_id));
        assert_eq!(run.subject.transcript_id(), None);
        assert_eq!(event.card_id, Some(card_id.0));
        assert_eq!(event.transcript_id, None);
        assert_eq!(event.pipeline, ["execute"]);
    }
}
