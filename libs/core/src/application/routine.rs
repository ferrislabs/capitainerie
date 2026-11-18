use capitainerie_common::{CoreError, RoutineId, RoutineTriggerId};
use capitainerie_macros::transactional;
use chrono::{DateTime, SubsecRound, Utc};

use super::CapitainerieUseCase;
use crate::domain::{
    agent_task::{AgentRunError, StartAgentRun},
    authorization::Subject,
    card::service::CreateTask,
    routine::{
        IssuedTrigger, Routine, RoutineListing, RoutineRun, RoutineRunView, Trigger,
        ports::RoutineRepository,
        service::{
            AddTrigger, CreateRoutine, Job, JobOutcome, RoutineRunService, RoutineScheduler,
            RoutineService, Triggered, UpdateRoutine, UpdateTrigger,
        },
    },
};

impl CapitainerieUseCase {
    #[transactional(routine, authz)]
    pub async fn list_routines(
        &self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<RoutineListing>, CoreError> {
        RoutineService::new(routine_repository, authz)
            .list(subject, archived)
            .await
    }

    #[transactional(routine, authz)]
    pub async fn get_routine(
        &self,
        subject: Subject,
        id: RoutineId,
    ) -> Result<RoutineListing, CoreError> {
        RoutineService::new(routine_repository, authz)
            .get(subject, id)
            .await
    }

    #[transactional(routine, crew_agent, project, authz)]
    pub async fn create_routine(
        &self,
        subject: Subject,
        command: CreateRoutine,
    ) -> Result<RoutineListing, CoreError> {
        let mut crew_agent_repository = crew_agent_repository;
        let mut project_repository = project_repository;
        RoutineService::new(routine_repository, authz)
            .create(
                subject,
                command,
                &mut crew_agent_repository,
                &mut project_repository,
                now(),
            )
            .await
    }

    #[transactional(routine, crew_agent, project, authz)]
    pub async fn update_routine(
        &self,
        subject: Subject,
        id: RoutineId,
        command: UpdateRoutine,
    ) -> Result<RoutineListing, CoreError> {
        let mut crew_agent_repository = crew_agent_repository;
        let mut project_repository = project_repository;
        RoutineService::new(routine_repository, authz)
            .update(
                subject,
                id,
                command,
                &mut crew_agent_repository,
                &mut project_repository,
                now(),
            )
            .await
    }

    #[transactional(routine, authz)]
    pub async fn archive_routine(
        &self,
        subject: Subject,
        id: RoutineId,
    ) -> Result<RoutineListing, CoreError> {
        RoutineService::new(routine_repository, authz)
            .archive(subject, id, now())
            .await
    }

    #[transactional(routine, authz)]
    pub async fn add_routine_trigger(
        &self,
        subject: Subject,
        id: RoutineId,
        command: AddTrigger,
    ) -> Result<IssuedTrigger, CoreError> {
        RoutineService::new(routine_repository, authz)
            .add_trigger(subject, id, command, now())
            .await
    }

    #[transactional(routine, authz)]
    pub async fn update_routine_trigger(
        &self,
        subject: Subject,
        id: RoutineId,
        trigger_id: RoutineTriggerId,
        command: UpdateTrigger,
    ) -> Result<RoutineListing, CoreError> {
        RoutineService::new(routine_repository, authz)
            .update_trigger(subject, id, trigger_id, command, now())
            .await
    }

    #[transactional(routine, authz)]
    pub async fn remove_routine_trigger(
        &self,
        subject: Subject,
        id: RoutineId,
        trigger_id: RoutineTriggerId,
    ) -> Result<RoutineListing, CoreError> {
        RoutineService::new(routine_repository, authz)
            .remove_trigger(subject, id, trigger_id)
            .await
    }

    #[transactional(routine, authz)]
    pub async fn rotate_routine_trigger_token(
        &self,
        subject: Subject,
        id: RoutineId,
        trigger_id: RoutineTriggerId,
    ) -> Result<IssuedTrigger, CoreError> {
        RoutineService::new(routine_repository, authz)
            .rotate_trigger_token(subject, id, trigger_id)
            .await
    }

    #[transactional(routine, routine_run, authz)]
    pub async fn list_routine_runs(
        &self,
        subject: Subject,
        id: RoutineId,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RoutineRunView>, i64), CoreError> {
        let mut routine_repository = routine_repository;
        let organization_id = subject.organization_id;
        let page = RoutineRunService::new(routine_run_repository, authz)
            .list(subject, id, limit, offset)
            .await?;
        if routine_repository
            .find(organization_id, id)
            .await?
            .is_none()
        {
            return Err(CoreError::NotFound);
        }
        Ok(page)
    }

    /// Creates a passage now, whatever the policies and the triggers say.
    pub async fn launch_routine(
        &self,
        subject: Subject,
        id: RoutineId,
    ) -> Result<RoutineRun, CoreError> {
        let job = self.store_launch(subject, id).await?;
        self.run_routine_job(job).await
    }

    /// The public hook: the token alone designates the routine and trigger.
    /// Returns the passage, or the queued or skipped one when `if_open` held it.
    pub async fn receive_routine_hook(&self, token: &str) -> Result<RoutineRun, CoreError> {
        match self.store_hook(token).await? {
            Triggered::Run(job) => self.run_routine_job(*job).await,
            Triggered::Held(Some(run)) => Ok(run),
            Triggered::Held(None) => Err(CoreError::Internal(
                "a held routine passage has no record".into(),
            )),
        }
    }

    /// One scheduler tick: claims the due triggers at `now`, then creates the
    /// card and starts the agent run of each passage they produce.
    pub async fn tick_routines(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<RoutineRun>, CoreError> {
        let jobs = self.claim_routines(now, limit).await?;
        let mut runs = Vec::with_capacity(jobs.len());
        for job in jobs {
            runs.push(self.run_routine_job(job).await?);
        }
        Ok(runs)
    }

    #[transactional(routine, routine_run, emitter)]
    async fn claim_routines(&self, now: DateTime<Utc>, limit: i64) -> Result<Vec<Job>, CoreError> {
        RoutineScheduler::new(routine_repository, routine_run_repository, emitter)
            .claim(now, limit)
            .await
    }

    #[transactional(routine, routine_run, emitter, authz)]
    async fn store_launch(&self, subject: Subject, id: RoutineId) -> Result<Job, CoreError> {
        let routine = RoutineService::new(routine_repository.clone(), authz)
            .for_launch(subject, id)
            .await?;
        RoutineScheduler::new(routine_repository, routine_run_repository, emitter)
            .launch(routine, now())
            .await
    }

    #[transactional(routine, routine_run, emitter, authz)]
    async fn store_hook(&self, token: &str) -> Result<Triggered, CoreError> {
        let (routine, trigger): (Routine, Trigger) =
            RoutineService::new(routine_repository.clone(), authz)
                .by_token(token)
                .await?;
        RoutineScheduler::new(routine_repository, routine_run_repository, emitter)
            .trigger(routine, trigger, now())
            .await
    }

    /// Creates the card on behalf of the routine's creator, then starts the
    /// agent's run exactly as `start_agent_run` does. A refused run keeps the
    /// card and records why.
    async fn run_routine_job(&self, job: Job) -> Result<RoutineRun, CoreError> {
        let routine = &job.routine;
        let subject = Subject {
            id: routine.created_by.clone(),
            organization_id: routine.organization_id,
            actor: None,
            is_admin: false,
        };
        let created = self
            .create_task(
                subject.clone(),
                CreateTask {
                    title: routine.card_title(job.run.scheduled_for),
                    description: routine.prompt.clone(),
                    priority: routine.priority,
                    client: None,
                    project: routine.project_id,
                    assignee_agent: Some(routine.agent_id),
                    parent: None,
                },
            )
            .await;
        let outcome = match created {
            Err(error) => JobOutcome::Failed(format!("tâche non créée : {error}")),
            Ok(card) => {
                let started = self
                    .start_agent_run(
                        subject,
                        card.id,
                        StartAgentRun {
                            expected_version: card.version,
                            allow_writes: routine.allow_writes,
                        },
                    )
                    .await;
                let (run_id, note) = match started {
                    Ok(started) => (Some(started.run_id), None),
                    Err(AgentRunError::Refused(refusal)) => (
                        None,
                        Some(format!(
                            "exécution non lancée ({}) : {refusal}",
                            refusal.code()
                        )),
                    ),
                    Err(AgentRunError::Core(error)) => {
                        (None, Some(format!("exécution non lancée : {error}")))
                    }
                };
                JobOutcome::Created {
                    card_id: card.id,
                    run_id,
                    note,
                }
            }
        };
        self.record_routine_job(job, outcome).await
    }

    #[transactional(routine, routine_run, emitter)]
    async fn record_routine_job(
        &self,
        job: Job,
        outcome: JobOutcome,
    ) -> Result<RoutineRun, CoreError> {
        RoutineScheduler::new(routine_repository, routine_run_repository, emitter)
            .record(job, outcome, now())
            .await
    }
}

fn now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(6)
}
