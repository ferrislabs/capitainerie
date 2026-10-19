use capitainerie_common::{CoreError, RunId, TranscriptId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        agent::SecretToken,
        authorization::{DecisionRecord, Subject, ports::DecisionLogRepository},
        delegation::ports::{DelegationVault, TokenExchanger},
        provider::ports::ModelAssignmentRepository,
        run::{
            RunOverview,
            active::ActiveRun,
            ports::StepRepository,
            service::{RunService, SubmitTranscript},
            step::Step,
        },
    },
    infrastructure::delegation::PgDelegationVault,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmittedTranscript {
    pub transcript_id: TranscriptId,
    pub run_id: RunId,
}

/// Everything that happened in a run, for its timeline.
#[derive(Debug, Clone)]
pub struct RunTimeline {
    pub run: RunOverview,
    pub steps: Vec<Step>,
    pub decisions: Vec<DecisionRecord>,
}

/// What the API needs to let an agent act for the submitting user later.
pub struct AgentDelegation<'a, X> {
    pub exchanger: &'a X,
    pub issuer: &'a str,
    pub user_token: &'a SecretToken,
    pub agents: &'a [String],
}

impl CapitainerieUseCase {
    /// Exchanges the user's token for each agent of the pipeline first, so an
    /// identity provider outage refuses the submission instead of queueing a
    /// run that could never write anything; then stores everything at once.
    pub async fn submit_transcript_delegated<X: TokenExchanger>(
        &self,
        command: SubmitTranscript,
        delegation: AgentDelegation<'_, X>,
    ) -> Result<SubmittedTranscript, CoreError> {
        let mut tokens = Vec::with_capacity(delegation.agents.len());
        for agent in delegation.agents {
            let token = delegation
                .exchanger
                .for_agent(delegation.issuer, delegation.user_token, agent)
                .await?;
            tokens.push((agent.clone(), token));
        }
        let cipher = self.cipher()?;
        self.submit_and_store(command, tokens, cipher).await
    }

    #[transactional(transcript, run, model_assignment, emitter, authz)]
    async fn submit_and_store(
        &self,
        command: SubmitTranscript,
        tokens: Vec<(String, crate::domain::delegation::DelegatedToken)>,
        cipher: &crate::infrastructure::crypto::SecretCipher,
    ) -> Result<SubmittedTranscript, CoreError> {
        let mut model_assignment_repository = model_assignment_repository;
        let models = model_assignment_repository
            .get(command.subject.organization_id)
            .await?;
        let (transcript, run) =
            RunService::new(transcript_repository, run_repository, emitter, authz)
                .submit_with_models(command, models)
                .await?;
        let mut vault = PgDelegationVault::new(&tx, cipher);
        for (agent, token) in &tokens {
            vault.store(run.id, agent, token).await?;
        }
        Ok(SubmittedTranscript {
            transcript_id: transcript.id,
            run_id: run.id,
        })
    }

    #[transactional(transcript, run, model_assignment, emitter, authz)]
    pub async fn submit_transcript(
        &self,
        command: SubmitTranscript,
    ) -> Result<SubmittedTranscript, CoreError> {
        let mut model_assignment_repository = model_assignment_repository;
        let models = model_assignment_repository
            .get(command.subject.organization_id)
            .await?;
        let (transcript, run) =
            RunService::new(transcript_repository, run_repository, emitter, authz)
                .submit_with_models(command, models)
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

    #[transactional(transcript, run, step, emitter, authz)]
    pub async fn list_active_runs(&self, subject: Subject) -> Result<Vec<ActiveRun>, CoreError> {
        let overviews = RunService::new(transcript_repository, run_repository, emitter, authz)
            .active(subject)
            .await?;
        let ids: Vec<RunId> = overviews.iter().map(|overview| overview.run.id).collect();
        let mut step_repository = step_repository;
        let steps = step_repository.list_for_runs(&ids).await?;
        let groups: Vec<&[Step]> = steps
            .chunk_by(|left, right| left.run_id == right.run_id)
            .collect();
        Ok(overviews
            .into_iter()
            .map(|overview| {
                let own = groups
                    .iter()
                    .find(|group| {
                        group
                            .first()
                            .is_some_and(|step| step.run_id == overview.run.id)
                    })
                    .copied()
                    .unwrap_or_default();
                ActiveRun::assemble(overview, own)
            })
            .collect())
    }

    #[transactional(transcript, run, step, decision_log, emitter, authz)]
    pub async fn run_timeline(
        &self,
        subject: Subject,
        id: RunId,
    ) -> Result<RunTimeline, CoreError> {
        let organization_id = subject.organization_id;
        let run = RunService::new(transcript_repository, run_repository, emitter, authz)
            .get(subject, id)
            .await?;
        let mut step_repository = step_repository;
        let mut decision_log_repository = decision_log_repository;
        Ok(RunTimeline {
            run,
            steps: step_repository.list(id).await?,
            decisions: decision_log_repository
                .list_for_run(organization_id, id)
                .await?,
        })
    }
}
