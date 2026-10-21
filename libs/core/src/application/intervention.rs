use capitainerie_common::{
    CardCommentId, CardId, CoreError, IntegrationInstanceId, OrganizationId, RunId,
};
use capitainerie_macros::transactional;
use chrono::{SubsecRound, Utc};

use super::{CapitainerieUseCase, execution::COMMENT_ID_FIELD};
use crate::{
    domain::{
        authorization::{AccessRequest, Action, ResourceKind, Subject, require},
        card::{
            Card, Column,
            ports::CardRepository,
            service::{CardService, MoveCard},
        },
        event::ports::EventEmitter,
        integration::{
            IntegrationInstance,
            confide::grants_for,
            intervention::{ConfideCard, IntervenableCard, Intervention, InterventionError},
            journal::INTEGRATOR_AGENT,
            ports::{IntegrationInstanceRepository, InterventionRepository},
        },
        provider::ports::ModelAssignmentRepository,
        run::{
            Run, RunSubject, StepKind,
            ports::{RunRepository, StepRepository},
        },
    },
    infrastructure::{
        card::postgres::PgCardRepository,
        integration::{
            interventions::PgInterventionRepository, postgres::PgIntegrationInstanceRepository,
        },
    },
};

#[derive(Debug, Clone)]
pub struct StartedIntervention {
    pub run_id: RunId,
    pub card: Card,
}

type Stored<T> = Result<Result<T, InterventionError>, CoreError>;

fn settled<T>(outcome: Result<T, InterventionError>) -> Stored<T> {
    match outcome {
        Err(InterventionError::Core(error)) => Err(error),
        other => Ok(other),
    }
}

impl CapitainerieUseCase {
    pub async fn confide_card_to_integrator(
        &self,
        subject: Subject,
        card_id: CardId,
        command: ConfideCard,
    ) -> Result<StartedIntervention, InterventionError> {
        self.store_intervention(subject, card_id, command).await?
    }

    #[transactional(card, card_comment, transcript, run, model_assignment, emitter, authz)]
    async fn store_intervention(
        &self,
        subject: Subject,
        card_id: CardId,
        command: ConfideCard,
    ) -> Stored<StartedIntervention> {
        let mut run_repository = run_repository;
        let mut model_assignment_repository = model_assignment_repository;
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(
                subject.clone(),
                Action::CardIntervene,
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
        let card = match IntervenableCard::try_from(card) {
            Ok(card) => card,
            Err(error) => return settled(Err(error)),
        };

        let mut instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        let mut instances: Vec<IntegrationInstance> = Vec::with_capacity(command.grants.len());
        for (instance_id, _) in &command.grants {
            if instances.iter().any(|loaded| loaded.id == *instance_id) {
                continue;
            }
            if let Some(instance) = instance_repository
                .find(organization_id, *instance_id)
                .await?
            {
                instances.push(instance);
            }
        }
        let grants = match grants_for(&card, &instances, &command) {
            Ok(grants) => grants,
            Err(refusal) => return Ok(Err(refusal.into())),
        };

        let moved = CardService::new(
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
                expected_version: command.expected_version,
                run_id: None,
            },
        )
        .await?;

        let now = Utc::now().trunc_subsecs(6);
        let (mut run, queued) =
            Run::queue_intervention(organization_id, card_id, subject.id.clone(), now);
        run.models = model_assignment_repository.get(organization_id).await?;
        run_repository.insert(&run).await?;
        emitter.emit(&queued)?;
        PgInterventionRepository::new(&tx)
            .insert(&Intervention {
                run_id: run.id,
                organization_id,
                card_id,
                client_id: card.client_id(),
                instructions: command.instructions.trim().to_owned(),
                grants,
                created_by: subject.id,
                created_at: now,
            })
            .await?;
        Ok(Ok(StartedIntervention {
            run_id: run.id,
            card: moved,
        }))
    }

    #[transactional(card, card_comment, transcript, run, step, emitter, authz)]
    pub async fn comment_card_as_integrator(
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
        if !matches!(run.subject, RunSubject::Intervention { card_id: owner } if owner == card_id) {
            return Err(CoreError::Invalid(
                "the run does not belong to this card".into(),
            ));
        }
        let mut intervene_step = step_repository
            .list(run_id)
            .await?
            .into_iter()
            .find(|step| step.kind == StepKind::Intervene);
        if let Some(posted) = intervene_step
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
                actor: Some(INTEGRATOR_AGENT.to_owned()),
                is_admin: false,
            },
            card_id,
            body,
            Some(run_id),
        )
        .await?;
        if let Some(step) = intervene_step.as_mut() {
            let mut output = step.output.take().unwrap_or_else(|| serde_json::json!({}));
            output[COMMENT_ID_FIELD] = serde_json::json!(comment.id.0.to_string());
            step.output = Some(output);
            step_repository.save(step).await?;
        }
        Ok(comment.id)
    }

    #[transactional()]
    pub async fn intervention_of(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Option<Intervention>, CoreError> {
        PgInterventionRepository::new(&tx)
            .find(organization_id, run_id)
            .await
    }

    #[transactional(authz)]
    pub async fn integration_instances_by_id(
        &self,
        subject: Subject,
        ids: &[IntegrationInstanceId],
    ) -> Result<Vec<IntegrationInstance>, CoreError> {
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(
                subject,
                Action::IntegrationRead,
                ResourceKind::IntegrationInstance,
                None,
            ),
        )
        .await?;
        let mut instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        let mut found: Vec<IntegrationInstance> = Vec::with_capacity(ids.len());
        for id in ids {
            if found.iter().any(|loaded| loaded.id == *id) {
                continue;
            }
            if let Some(instance) = instance_repository.find(organization_id, *id).await? {
                found.push(instance);
            }
        }
        Ok(found)
    }
}
