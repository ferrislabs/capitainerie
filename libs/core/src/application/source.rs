use capitainerie_common::{CoreError, SourceId};
use capitainerie_macros::transactional;
use chrono::{SubsecRound, Utc};
use serde_json::{Value, json};

use super::CapitainerieUseCase;
use crate::domain::{
    agent_task::{AgentRunError, StartAgentRun},
    authorization::Subject,
    card::{attributes::Priority, service::CreateTask},
    source::{
        IssuedSource, Source, SourceEvent, SourceEventStatus, SourceListing, card_description,
        card_title,
        ports::SourceRepository,
        service::{
            CreateSource, EventOutcome, HookRefusal, SourceEventService, SourceService,
            UpdateSource,
        },
    },
};

type Received = Result<Result<(Source, SourceEvent), HookRefusal>, CoreError>;

impl CapitainerieUseCase {
    #[transactional(source, authz)]
    pub async fn list_sources(
        &self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<SourceListing>, CoreError> {
        SourceService::new(source_repository, authz)
            .list(subject, archived)
            .await
    }

    #[transactional(source, authz)]
    pub async fn get_source(
        &self,
        subject: Subject,
        id: SourceId,
    ) -> Result<SourceListing, CoreError> {
        SourceService::new(source_repository, authz)
            .get(subject, id)
            .await
    }

    #[transactional(source, crew_agent, project, authz)]
    pub async fn create_source(
        &self,
        subject: Subject,
        command: CreateSource,
    ) -> Result<IssuedSource, CoreError> {
        let mut crew_agent_repository = crew_agent_repository;
        let mut project_repository = project_repository;
        SourceService::new(source_repository, authz)
            .create(
                subject,
                command,
                &mut crew_agent_repository,
                &mut project_repository,
            )
            .await
    }

    #[transactional(source, crew_agent, project, authz)]
    pub async fn update_source(
        &self,
        subject: Subject,
        id: SourceId,
        command: UpdateSource,
    ) -> Result<SourceListing, CoreError> {
        let mut crew_agent_repository = crew_agent_repository;
        let mut project_repository = project_repository;
        SourceService::new(source_repository, authz)
            .update(
                subject,
                id,
                command,
                &mut crew_agent_repository,
                &mut project_repository,
            )
            .await
    }

    #[transactional(source, authz)]
    pub async fn archive_source(
        &self,
        subject: Subject,
        id: SourceId,
    ) -> Result<SourceListing, CoreError> {
        SourceService::new(source_repository, authz)
            .archive(subject, id)
            .await
    }

    #[transactional(source, authz)]
    pub async fn rotate_source_token(
        &self,
        subject: Subject,
        id: SourceId,
    ) -> Result<IssuedSource, CoreError> {
        SourceService::new(source_repository, authz)
            .rotate_token(subject, id)
            .await
    }

    #[transactional(source, source_event, emitter, authz)]
    pub async fn list_source_events(
        &self,
        subject: Subject,
        id: SourceId,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<SourceEvent>, i64), CoreError> {
        let mut source_repository = source_repository;
        let organization_id = subject.organization_id;
        let page = SourceEventService::new(source_event_repository, emitter, authz)
            .list(subject, id, limit, offset)
            .await?;
        if source_repository.find(organization_id, id).await?.is_none() {
            return Err(CoreError::NotFound);
        }
        Ok(page)
    }

    /// The public hook: the token alone designates the source. The payload is
    /// untrusted and only ever lands in the card's description.
    pub async fn receive_source_event(
        &self,
        token: &str,
        payload: Value,
    ) -> Result<Result<SourceEvent, HookRefusal>, CoreError> {
        match self.store_hook_event(token, payload).await? {
            Ok((source, event)) => Ok(Ok(self.process_source_event(&source, event).await?)),
            Err(refusal) => Ok(Err(refusal)),
        }
    }

    /// Sends a sample event through the same path as the hook.
    pub async fn test_source(
        &self,
        subject: Subject,
        id: SourceId,
    ) -> Result<Result<SourceEvent, HookRefusal>, CoreError> {
        match self.store_test_event(subject, id).await? {
            Ok((source, event)) => Ok(Ok(self.process_source_event(&source, event).await?)),
            Err(refusal) => Ok(Err(refusal)),
        }
    }

    #[transactional(source, source_event, emitter, authz)]
    async fn store_hook_event(&self, token: &str, payload: Value) -> Received {
        let source = SourceService::new(source_repository, authz)
            .by_token(token)
            .await?;
        Ok(
            SourceEventService::new(source_event_repository, emitter, authz)
                .receive(&source, payload, Utc::now().trunc_subsecs(6))
                .await?
                .map(|event| (source, event)),
        )
    }

    #[transactional(source, source_event, emitter, authz)]
    async fn store_test_event(&self, subject: Subject, id: SourceId) -> Received {
        let source = SourceService::new(source_repository, authz)
            .for_test(subject, id)
            .await?;
        let payload = json!({
            "title": format!("Test de la source {}", source.name),
            "text": "Événement d'essai envoyé depuis la console.",
            "test": true,
        });
        Ok(
            SourceEventService::new(source_event_repository, emitter, authz)
                .receive(&source, payload, Utc::now().trunc_subsecs(6))
                .await?
                .map(|event| (source, event)),
        )
    }

    /// Creates the card on behalf of the source's creator, then, when the
    /// source asks for it, queues its default agent's run exactly as
    /// `start_agent_run` does: the run uses the creator's stored provider and
    /// connector sessions, never a live token.
    async fn process_source_event(
        &self,
        source: &Source,
        event: SourceEvent,
    ) -> Result<SourceEvent, CoreError> {
        let subject = Subject {
            id: source.created_by.clone(),
            organization_id: source.organization_id,
            actor: None,
            is_admin: false,
        };
        let created = self
            .create_task(
                subject.clone(),
                CreateTask {
                    title: card_title(&event.payload, &source.name),
                    description: card_description(&event.payload, &source.name),
                    priority: Priority::default(),
                    client: None,
                    project: source.project_id,
                    assignee_agent: source.default_agent_id,
                    parent: None,
                },
            )
            .await;
        let outcome = match created {
            Err(error) => EventOutcome {
                status: SourceEventStatus::Failed,
                card_id: None,
                run_id: None,
                note: Some(format!("carte non créée : {error}")),
            },
            Ok(card) => {
                let (run_id, note) = match (source.auto_run, source.default_agent_id) {
                    (false, _) => (None, None),
                    (true, None) => (
                        None,
                        Some("exécution non lancée : la source n'a pas d'agent par défaut".into()),
                    ),
                    (true, Some(_)) => match self
                        .start_agent_run(
                            subject,
                            card.id,
                            StartAgentRun {
                                expected_version: card.version,
                                allow_writes: source.allow_writes,
                            },
                        )
                        .await
                    {
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
                    },
                };
                EventOutcome {
                    status: SourceEventStatus::CardCreated,
                    card_id: Some(card.id),
                    run_id,
                    note,
                }
            }
        };
        self.record_source_outcome(event, outcome).await
    }

    #[transactional(source_event, emitter, authz)]
    async fn record_source_outcome(
        &self,
        event: SourceEvent,
        outcome: EventOutcome,
    ) -> Result<SourceEvent, CoreError> {
        SourceEventService::new(source_event_repository, emitter, authz)
            .record(event, outcome)
            .await
    }
}
