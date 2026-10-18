use capitainerie_common::{CardCommentId, CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::transactional;
use chrono::{SubsecRound, Utc};
use uuid::Uuid;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::{AccessRequest, Action, ResourceKind, Subject, require},
        card::{
            Card, Column,
            ports::CardRepository,
            service::{CardService, MoveCard},
        },
        event::ports::EventEmitter,
        execution::{
            EXECUTOR_AGENT, ExecutableCard, GithubInstallation, RepositoryRef, TaskGrant,
            ports::{GithubInstallationRepository, TaskGrantRepository},
        },
        provider::ports::ModelAssignmentRepository,
        run::{
            Run, StepKind,
            ports::{RunRepository, StepRepository},
        },
    },
    infrastructure::card::postgres::PgCardRepository,
};

pub(super) const COMMENT_ID_FIELD: &str = "comment_id";

#[derive(Debug, Clone)]
pub struct StartedExecution {
    pub run_id: RunId,
    pub card: Card,
}

impl CapitainerieUseCase {
    /// The read-only half of `execute_card`: the policy, the card, its state
    /// and its executions, so a handler can refuse before it calls GitHub.
    /// `execute_card` checks everything again inside its writing transaction.
    #[transactional(card, run, authz)]
    pub async fn check_card_executable(
        &self,
        subject: Subject,
        card_id: CardId,
    ) -> Result<(), CoreError> {
        let mut card_repository = card_repository;
        let mut run_repository = run_repository;
        require(
            authz,
            AccessRequest::new(
                subject.clone(),
                Action::CardExecute,
                ResourceKind::Card,
                Some(card_id.0),
            ),
        )
        .await?;
        let card = card_repository
            .find(subject.organization_id, card_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if run_repository.has_active_execution(card_id).await? {
            return Err(CoreError::Conflict("execution_already_active".into()));
        }
        ExecutableCard::try_from(card)?;
        Ok(())
    }

    #[transactional(
        card,
        card_comment,
        transcript,
        run,
        model_assignment,
        github_installation,
        emitter,
        authz
    )]
    pub async fn execute_card(
        &self,
        subject: Subject,
        card_id: CardId,
        repository: RepositoryRef,
        base_branch: String,
        expected_version: i32,
    ) -> Result<StartedExecution, CoreError> {
        let mut run_repository = run_repository;
        let mut model_assignment_repository = model_assignment_repository;
        let mut github_installation_repository = github_installation_repository;
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(
                subject.clone(),
                Action::CardExecute,
                ResourceKind::Card,
                Some(card_id.0),
            ),
        )
        .await?;

        let card = PgCardRepository::new(&tx)
            .find(organization_id, card_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if run_repository.has_active_execution(card_id).await? {
            return Err(CoreError::Conflict("execution_already_active".into()));
        }
        ExecutableCard::try_from(card)?;
        if github_installation_repository
            .get(organization_id)
            .await?
            .is_none()
        {
            return Err(CoreError::Conflict("github_not_connected".into()));
        }

        let card = CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .move_card(
            subject.clone(),
            card_id,
            MoveCard {
                to: Column::InProgress,
                expected_version,
                run_id: None,
            },
        )
        .await?;

        let (mut run, queued) = Run::queue_execution(
            organization_id,
            card_id,
            repository,
            base_branch,
            subject.id,
            Utc::now(),
        );
        run.models = model_assignment_repository.get(organization_id).await?;
        run_repository.insert(&run).await?;
        emitter.emit(&queued)?;
        Ok(StartedExecution {
            run_id: run.id,
            card,
        })
    }

    /// The executor's report on its card. The worker writes it itself, as the
    /// human who handed the card over with the executor as the acting agent;
    /// the policy decision is audited with the run. Posting twice is a no-op:
    /// the id of the comment lives in the Execute step, in the same
    /// transaction as the comment.
    #[transactional(card, card_comment, transcript, run, step, emitter, authz)]
    pub async fn comment_card_as_executor(
        &self,
        run_id: RunId,
        card_id: CardId,
        body: String,
    ) -> Result<CardCommentId, CoreError> {
        let mut run_repository = run_repository;
        let mut step_repository = step_repository;
        let run = run_repository
            .lock(run_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if run.subject.card_id() != Some(card_id) {
            return Err(CoreError::Invalid(
                "the run does not belong to this card".into(),
            ));
        }
        let mut execute_step = step_repository
            .list(run_id)
            .await?
            .into_iter()
            .find(|step| step.kind == StepKind::Execute);
        if let Some(posted) = execute_step
            .as_ref()
            .and_then(|step| step.output.as_ref())
            .and_then(|output| output[COMMENT_ID_FIELD].as_str())
            .and_then(|id| id.parse::<uuid::Uuid>().ok())
        {
            return Ok(CardCommentId(posted));
        }
        let comment = CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .comment(
            Subject {
                id: run.initiated_by.clone(),
                organization_id: run.organization_id,
                actor: Some(EXECUTOR_AGENT.to_owned()),
                is_admin: false,
            },
            card_id,
            body,
            Some(run_id),
        )
        .await?;
        if let Some(step) = execute_step.as_mut() {
            let mut output = step.output.take().unwrap_or_else(|| serde_json::json!({}));
            output[COMMENT_ID_FIELD] = serde_json::json!(comment.id.0.to_string());
            step.output = Some(output);
            step_repository.save(step).await?;
        }
        Ok(comment.id)
    }

    /// Asks the policy decision point on behalf of a handler that must refuse
    /// before it does anything else; the decision is audited either way.
    #[transactional(authz)]
    pub async fn authorize(
        &self,
        subject: Subject,
        action: Action,
        kind: ResourceKind,
        id: Option<Uuid>,
    ) -> Result<(), CoreError> {
        let _ = &tx;
        require(authz, AccessRequest::new(subject, action, kind, id)).await
    }

    /// Any member may know whether GitHub is connected; the organization
    /// scopes the read.
    #[transactional(github_installation)]
    pub async fn github_installation(
        &self,
        subject: Subject,
    ) -> Result<Option<GithubInstallation>, CoreError> {
        let mut github_installation_repository = github_installation_repository;
        github_installation_repository
            .get(subject.organization_id)
            .await
    }

    #[transactional(github_installation, authz)]
    pub async fn connect_github(
        &self,
        subject: Subject,
        installation_id: i64,
        account_login: String,
    ) -> Result<GithubInstallation, CoreError> {
        let mut github_installation_repository = github_installation_repository;
        require(
            authz,
            AccessRequest::new(
                subject.clone(),
                Action::GithubManage,
                ResourceKind::GithubInstallation,
                None,
            ),
        )
        .await?;
        let installation = GithubInstallation {
            organization_id: subject.organization_id,
            installation_id,
            account_login,
            created_by: subject.id,
            // Postgres keeps microseconds: the value returned must equal the stored one.
            created_at: Utc::now().trunc_subsecs(6),
        };
        github_installation_repository
            .save(&installation)
            .await
            .map_err(|error| match error {
                CoreError::Conflict(_) => {
                    CoreError::Conflict("installation_already_connected".into())
                }
                other => other,
            })?;
        Ok(installation)
    }

    #[transactional(github_installation, authz)]
    pub async fn disconnect_github(&self, subject: Subject) -> Result<(), CoreError> {
        let mut github_installation_repository = github_installation_repository;
        require(
            authz,
            AccessRequest::new(
                subject.clone(),
                Action::GithubManage,
                ResourceKind::GithubInstallation,
                None,
            ),
        )
        .await?;
        github_installation_repository
            .delete(subject.organization_id)
            .await
    }

    /// For the worker and the proxy, which act for no user: no policy
    /// decision, the organization of the run scopes the read.
    #[transactional(github_installation)]
    pub async fn github_installation_for(
        &self,
        organization_id: OrganizationId,
    ) -> Result<Option<GithubInstallation>, CoreError> {
        let mut github_installation_repository = github_installation_repository;
        github_installation_repository.get(organization_id).await
    }

    #[transactional(task_grant)]
    pub async fn create_task_grant(
        &self,
        run_id: RunId,
        token_sha256: [u8; 32],
        grant: TaskGrant,
    ) -> Result<(), CoreError> {
        let mut task_grant_repository = task_grant_repository;
        task_grant_repository
            .insert(token_sha256, &TaskGrant { run_id, ..grant })
            .await
    }

    #[transactional(task_grant)]
    pub async fn find_task_grant(
        &self,
        token_sha256: [u8; 32],
    ) -> Result<Option<TaskGrant>, CoreError> {
        let mut task_grant_repository = task_grant_repository;
        task_grant_repository.find(token_sha256).await
    }

    #[transactional(task_grant)]
    pub async fn revoke_task_grants(&self, run_id: RunId) -> Result<(), CoreError> {
        let mut task_grant_repository = task_grant_repository;
        task_grant_repository.revoke(run_id, Utc::now()).await
    }
}
