use capitainerie_common::{CoreError, ProjectId};
use capitainerie_macros::transactional;
use chrono::{SubsecRound, Utc};

use super::{CapitainerieUseCase, execution::StartedExecution};
use crate::{
    domain::{
        authorization::{AccessRequest, Action, ResourceKind, Subject, require},
        card::{
            Column,
            link::{CardLinkService, LinkCard},
            service::{CardService, CreateCard, MoveCard},
        },
        client::links::Change,
        event::ports::EventEmitter,
        execution::{RepositoryRef, ports::GithubInstallationRepository},
        project::ports::ProjectRepository,
        provider::ports::ModelAssignmentRepository,
        run::{Run, ports::RunRepository},
        worksite::{
            MAX_WORKSITES_PER_LAUNCH, WorksiteDraft, WorksiteIssue, WorksiteSummary,
            ports::WorksiteRepository,
        },
    },
    infrastructure::{
        card::postgres::{PgCardCommentRepository, PgCardRepository},
        client::postgres::PgClientRepository,
        project::postgres::PgProjectRepository,
        transcript::postgres::PgTranscriptRepository,
    },
};

impl CapitainerieUseCase {
    /// Opens every worksite in one transaction: each draft becomes a card of
    /// the project, validated and handed to the executor, with the run queued.
    /// Any refusal rolls back the whole batch.
    #[transactional(
        project,
        run,
        model_assignment,
        github_installation,
        worksite,
        emitter,
        authz
    )]
    pub async fn launch_worksites(
        &self,
        subject: Subject,
        project_id: ProjectId,
        repository: RepositoryRef,
        base_branch: String,
        drafts: Vec<WorksiteDraft>,
    ) -> Result<Vec<StartedExecution>, CoreError> {
        let mut project_repository = project_repository;
        let mut run_repository = run_repository;
        let mut model_assignment_repository = model_assignment_repository;
        let mut github_installation_repository = github_installation_repository;
        let mut worksite_repository = worksite_repository;
        if drafts.is_empty() {
            return Err(CoreError::Invalid("no worksite to open".into()));
        }
        if drafts.len() > MAX_WORKSITES_PER_LAUNCH {
            return Err(CoreError::Invalid(format!(
                "at most {MAX_WORKSITES_PER_LAUNCH} worksites at once"
            )));
        }
        let organization_id = subject.organization_id;
        project_repository
            .find(organization_id, project_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if github_installation_repository
            .get(organization_id)
            .await?
            .is_none()
        {
            return Err(CoreError::Conflict("github_not_connected".into()));
        }
        let models = model_assignment_repository.get(organization_id).await?;

        let mut started = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let card = CardService::new(
                PgCardRepository::new(&tx),
                PgCardCommentRepository::new(&tx),
                PgTranscriptRepository::new(&tx),
                emitter,
                authz,
            )
            .create(
                subject.clone(),
                CreateCard {
                    title: draft.title,
                    description: draft.description,
                    provenance: None,
                    run_id: None,
                    attributes: Default::default(),
                },
            )
            .await?;
            let card = CardLinkService::new(
                PgCardRepository::new(&tx),
                PgClientRepository::new(&tx),
                PgProjectRepository::new(&tx),
                emitter,
                authz,
            )
            .link(
                subject.clone(),
                card.id,
                LinkCard {
                    expected_version: card.version,
                    client: Change::Keep,
                    project: Change::Set(project_id),
                },
            )
            .await?;
            let mut card = card;
            for column in [Column::Todo, Column::InProgress] {
                require(
                    authz,
                    AccessRequest::new(
                        subject.clone(),
                        Action::CardExecute,
                        ResourceKind::Card,
                        Some(card.id.0),
                    ),
                )
                .await?;
                card = CardService::new(
                    PgCardRepository::new(&tx),
                    PgCardCommentRepository::new(&tx),
                    PgTranscriptRepository::new(&tx),
                    emitter,
                    authz,
                )
                .move_card(
                    subject.clone(),
                    card.id,
                    MoveCard {
                        to: column,
                        expected_version: card.version,
                        run_id: None,
                    },
                )
                .await?;
            }
            if let Some(issue) = draft.issue {
                worksite_repository
                    .insert_issue(&WorksiteIssue {
                        card_id: card.id,
                        organization_id,
                        repository: repository.clone(),
                        number: issue.number,
                        url: issue.url,
                        created_at: Utc::now().round_subsecs(6),
                    })
                    .await?;
            }
            let (mut run, queued) = Run::queue_execution(
                organization_id,
                card.id,
                repository.clone(),
                base_branch.clone(),
                subject.id.clone(),
                Utc::now(),
            );
            run.models = models.clone();
            run_repository.insert(&run).await?;
            emitter.emit(&queued)?;
            started.push(StartedExecution {
                run_id: run.id,
                card,
            });
        }
        Ok(started)
    }

    #[transactional(project, worksite, authz)]
    pub async fn list_worksites(
        &self,
        subject: Subject,
        project_id: ProjectId,
    ) -> Result<Vec<WorksiteSummary>, CoreError> {
        let mut project_repository = project_repository;
        let mut worksite_repository = worksite_repository;
        require(
            authz,
            AccessRequest::new(subject.clone(), Action::CardRead, ResourceKind::Card, None),
        )
        .await?;
        project_repository
            .find(subject.organization_id, project_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        worksite_repository
            .list(subject.organization_id, project_id)
            .await
    }

    /// The issues of `repository` among `numbers` that already have a card.
    #[transactional(worksite, authz)]
    pub async fn imported_issues(
        &self,
        subject: Subject,
        repository: RepositoryRef,
        numbers: Vec<u64>,
    ) -> Result<Vec<WorksiteIssue>, CoreError> {
        let mut worksite_repository = worksite_repository;
        require(
            authz,
            AccessRequest::new(subject.clone(), Action::CardRead, ResourceKind::Card, None),
        )
        .await?;
        worksite_repository
            .issues(subject.organization_id, &repository, &numbers)
            .await
    }
}
