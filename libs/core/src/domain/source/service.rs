use capitainerie_common::{
    AgentId, CardId, CoreError, OrganizationId, ProjectId, RunId, SourceEventId, SourceId,
};
use capitainerie_events::source::{SourceEventProcessed, SourceEventReceived};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use super::{
    DESCRIPTION_MAX_CHARS_SOURCE, ERROR_MAX_CHARS, EVENTS_PER_MINUTE, IssuedSource, Source,
    SourceConfig, SourceEvent, SourceEventStatus, SourceKind, SourceListing, TokenHash,
    WebhookToken, card_title, check_payload,
    ports::{SourceEventRepository, SourceRepository},
    truncate,
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    card::{bounded_text, relations::check_agent},
    client::links::Change,
    crew::{Slug, ports::CrewRepository},
    event::ports::EventEmitter,
    label::Name,
    project::ports::ProjectRepository,
};

#[derive(Debug, Clone)]
pub struct CreateSource {
    pub slug: String,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub enabled: bool,
    pub config: Option<Value>,
    pub default_agent_id: Option<AgentId>,
    pub project_id: Option<ProjectId>,
    pub auto_run: bool,
    pub allow_writes: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSource {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<Value>,
    pub default_agent_id: Change<AgentId>,
    pub project_id: Change<ProjectId>,
    pub auto_run: Option<bool>,
    pub allow_writes: Option<bool>,
    pub archived: Option<bool>,
}

impl UpdateSource {
    fn changes_nothing(&self) -> bool {
        self.slug.is_none()
            && self.name.is_none()
            && self.description.is_none()
            && self.enabled.is_none()
            && self.config.is_none()
            && matches!(self.default_agent_id, Change::Keep)
            && matches!(self.project_id, Change::Keep)
            && self.auto_run.is_none()
            && self.allow_writes.is_none()
            && self.archived.is_none()
    }
}

/// What became of an event once handled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventOutcome {
    pub status: SourceEventStatus,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    /// Why no card was created, or why no run was started.
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HookRefusal {
    #[error("the source received more than {EVENTS_PER_MINUTE} events in the last minute")]
    RateLimited,
}

pub struct SourceService<'a, R, A> {
    sources: R,
    authorizer: &'a A,
}

impl<'a, R, A> SourceService<'a, R, A>
where
    R: SourceRepository,
    A: Authorizer,
{
    pub fn new(sources: R, authorizer: &'a A) -> Self {
        Self {
            sources,
            authorizer,
        }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<SourceListing>, CoreError> {
        let organization_id = subject.organization_id;
        require_source(self.authorizer, subject, Action::SourceRead, None).await?;
        self.sources.list(organization_id, archived).await
    }

    pub async fn get(mut self, subject: Subject, id: SourceId) -> Result<SourceListing, CoreError> {
        let organization_id = subject.organization_id;
        require_source(self.authorizer, subject, Action::SourceRead, Some(id)).await?;
        self.found(organization_id, id).await
    }

    pub async fn create<C, P>(
        mut self,
        subject: Subject,
        command: CreateSource,
        crew: &mut C,
        projects: &mut P,
    ) -> Result<IssuedSource, CoreError>
    where
        C: CrewRepository,
        P: ProjectRepository,
    {
        let organization_id = subject.organization_id;
        require_source(self.authorizer, subject.clone(), Action::SourceManage, None).await?;
        if let Some(agent) = command.default_agent_id {
            check_agent(crew, organization_id, agent).await?;
        }
        if let Some(project) = command.project_id {
            check_project(projects, organization_id, project).await?;
        }
        let token = WebhookToken::generate()?;
        let now = Utc::now();
        let source = Source {
            id: SourceId::new(),
            organization_id,
            slug: Slug::try_from(command.slug)?,
            name: Name::try_from(command.name)?.as_str().to_owned(),
            kind: SourceKind::try_from(command.kind.as_str())?,
            description: description(command.description)?,
            enabled: command.enabled,
            config: command
                .config
                .map_or_else(|| Ok(SourceConfig::empty()), SourceConfig::try_from)?,
            webhook_token_hash: token.hash(),
            default_agent_id: command.default_agent_id,
            project_id: command.project_id,
            auto_run: command.auto_run,
            allow_writes: command.allow_writes,
            archived: false,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
        };
        self.sources.insert(&source).await?;
        let listing = self.found(organization_id, source.id).await?;
        Ok(IssuedSource { listing, token })
    }

    pub async fn update<C, P>(
        mut self,
        subject: Subject,
        id: SourceId,
        command: UpdateSource,
        crew: &mut C,
        projects: &mut P,
    ) -> Result<SourceListing, CoreError>
    where
        C: CrewRepository,
        P: ProjectRepository,
    {
        let organization_id = subject.organization_id;
        require_source(self.authorizer, subject, Action::SourceManage, Some(id)).await?;
        let mut source = self.found(organization_id, id).await?.source;
        if command.changes_nothing() {
            return Err(CoreError::Invalid(
                "nothing to update: give at least one field".into(),
            ));
        }
        if let Some(slug) = command.slug {
            source.slug = Slug::try_from(slug)?;
        }
        if let Some(name) = command.name {
            source.name = Name::try_from(name)?.as_str().to_owned();
        }
        if let Some(value) = command.description {
            source.description = description(value)?;
        }
        if let Some(enabled) = command.enabled {
            source.enabled = enabled;
        }
        if let Some(config) = command.config {
            source.config = SourceConfig::try_from(config)?;
        }
        match command.default_agent_id {
            Change::Keep => {}
            Change::Clear => source.default_agent_id = None,
            Change::Set(agent) => {
                check_agent(crew, organization_id, agent).await?;
                source.default_agent_id = Some(agent);
            }
        }
        match command.project_id {
            Change::Keep => {}
            Change::Clear => source.project_id = None,
            Change::Set(project) => {
                check_project(projects, organization_id, project).await?;
                source.project_id = Some(project);
            }
        }
        if let Some(auto_run) = command.auto_run {
            source.auto_run = auto_run;
        }
        if let Some(allow_writes) = command.allow_writes {
            source.allow_writes = allow_writes;
        }
        if let Some(archived) = command.archived {
            source.archived = archived;
        }
        self.save(source).await
    }

    pub async fn archive(
        mut self,
        subject: Subject,
        id: SourceId,
    ) -> Result<SourceListing, CoreError> {
        let organization_id = subject.organization_id;
        require_source(self.authorizer, subject, Action::SourceManage, Some(id)).await?;
        let found = self.found(organization_id, id).await?;
        if found.source.archived {
            return Ok(found);
        }
        let mut source = found.source;
        source.archived = true;
        self.save(source).await
    }

    /// Issues a new token; the previous one stops working at once.
    pub async fn rotate_token(
        mut self,
        subject: Subject,
        id: SourceId,
    ) -> Result<IssuedSource, CoreError> {
        let organization_id = subject.organization_id;
        require_source(self.authorizer, subject, Action::SourceManage, Some(id)).await?;
        let mut source = self.found(organization_id, id).await?.source;
        let token = WebhookToken::generate()?;
        source.webhook_token_hash = token.hash();
        let listing = self.save(source).await?;
        Ok(IssuedSource { listing, token })
    }

    /// The source a test event goes to: managing it is required.
    pub async fn for_test(mut self, subject: Subject, id: SourceId) -> Result<Source, CoreError> {
        let organization_id = subject.organization_id;
        require_source(self.authorizer, subject, Action::SourceManage, Some(id)).await?;
        let source = self.found(organization_id, id).await?.source;
        if source.archived {
            return Err(CoreError::Invalid("the source is archived".into()));
        }
        Ok(source)
    }

    /// The source a presented token designates, if it accepts events. Every
    /// other case, unknown or disabled alike, is a `NotFound`.
    pub async fn by_token(mut self, token: &str) -> Result<Source, CoreError> {
        if !WebhookToken::plausible(token) {
            return Err(CoreError::NotFound);
        }
        self.sources
            .find_by_token(&TokenHash::of(token))
            .await?
            .filter(Source::accepts_events)
            .ok_or(CoreError::NotFound)
    }

    async fn found(
        &mut self,
        organization_id: OrganizationId,
        id: SourceId,
    ) -> Result<SourceListing, CoreError> {
        self.sources
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    async fn save(&mut self, mut source: Source) -> Result<SourceListing, CoreError> {
        source.updated_at = Utc::now();
        self.sources.update(&source).await?;
        self.found(source.organization_id, source.id).await
    }
}

pub struct SourceEventService<'a, R, E, A> {
    events: R,
    emitter: &'a E,
    authorizer: &'a A,
}

impl<'a, R, E, A> SourceEventService<'a, R, E, A>
where
    R: SourceEventRepository,
    E: EventEmitter,
    A: Authorizer,
{
    pub fn new(events: R, emitter: &'a E, authorizer: &'a A) -> Self {
        Self {
            events,
            emitter,
            authorizer,
        }
    }

    /// Stores an event a source received, unless the source is over its rate.
    pub async fn receive(
        mut self,
        source: &Source,
        payload: Value,
        now: DateTime<Utc>,
    ) -> Result<Result<SourceEvent, HookRefusal>, CoreError> {
        check_payload(&payload)?;
        let recent = self
            .events
            .count_since(source.id, now - Duration::minutes(1))
            .await?;
        if recent >= EVENTS_PER_MINUTE {
            return Ok(Err(HookRefusal::RateLimited));
        }
        let event = SourceEvent {
            id: SourceEventId::new(),
            organization_id: source.organization_id,
            source_id: source.id,
            received_at: now,
            status: SourceEventStatus::Received,
            summary: card_title(&payload, &source.name),
            payload,
            card_id: None,
            run_id: None,
            error: None,
        };
        self.events.insert(&event).await?;
        self.emitter.emit(&SourceEventReceived {
            event_id: event.id.0,
            source_id: source.id.0,
            organization_id: source.organization_id.0,
        })?;
        Ok(Ok(event))
    }

    pub async fn record(
        mut self,
        mut event: SourceEvent,
        outcome: EventOutcome,
    ) -> Result<SourceEvent, CoreError> {
        event.status = outcome.status;
        event.card_id = outcome.card_id;
        event.run_id = outcome.run_id;
        event.error = outcome.note.map(|note| truncate(&note, ERROR_MAX_CHARS));
        self.events.save_outcome(&event).await?;
        self.emitter.emit(&SourceEventProcessed {
            event_id: event.id.0,
            source_id: event.source_id.0,
            organization_id: event.organization_id.0,
            status: event.status.as_str().to_owned(),
            card_id: event.card_id.map(|id| id.0),
            run_id: event.run_id.map(|id| id.0),
        })?;
        Ok(event)
    }

    /// The source must be readable and in the caller's organization; the
    /// caller checks it exists.
    pub async fn list(
        mut self,
        subject: Subject,
        source_id: SourceId,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<SourceEvent>, i64), CoreError> {
        let organization_id = subject.organization_id;
        require_source(
            self.authorizer,
            subject,
            Action::SourceRead,
            Some(source_id),
        )
        .await?;
        self.events
            .list(
                organization_id,
                source_id,
                limit.clamp(1, 100),
                offset.max(0),
            )
            .await
    }
}

async fn require_source<A: Authorizer>(
    authorizer: &A,
    subject: Subject,
    action: Action,
    id: Option<SourceId>,
) -> Result<(), CoreError> {
    require(
        authorizer,
        AccessRequest::new(subject, action, ResourceKind::Source, id.map(|id| id.0)),
    )
    .await
}

async fn check_project<P: ProjectRepository>(
    projects: &mut P,
    organization_id: OrganizationId,
    id: ProjectId,
) -> Result<(), CoreError> {
    match projects.find(organization_id, id).await? {
        None => Err(CoreError::Invalid(
            "the project is not in the organization".into(),
        )),
        Some(project) if project.archived_at.is_some() => {
            Err(CoreError::Invalid("the project is archived".into()))
        }
        Some(_) => Ok(()),
    }
}

fn description(value: String) -> Result<String, CoreError> {
    bounded_text(
        value.trim().to_owned(),
        DESCRIPTION_MAX_CHARS_SOURCE,
        "description",
        true,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use capitainerie_events::DomainEvent;
    use serde_json::json;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        crew::{ports::MockCrewRepository, tests::agent},
        project::ports::MockProjectRepository,
        source::{
            ports::{MockSourceEventRepository, MockSourceRepository},
            tests::source,
        },
    };

    #[derive(Default)]
    struct RecordingEmitter(Mutex<Vec<String>>);

    impl EventEmitter for RecordingEmitter {
        fn emit<Ev: DomainEvent>(&self, _event: &Ev) -> Result<(), CoreError> {
            self.0.lock().unwrap().push(Ev::KIND.to_owned());
            Ok(())
        }
    }

    fn human(organization_id: OrganizationId) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id,
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

    fn command(slug: &str) -> CreateSource {
        CreateSource {
            slug: slug.into(),
            name: " Chatwoot ".into(),
            kind: "webhook".into(),
            description: String::new(),
            enabled: true,
            config: Some(json!({"patterns": ["message_created"]})),
            default_agent_id: None,
            project_id: None,
            auto_run: false,
            allow_writes: false,
        }
    }

    fn storing() -> (MockSourceRepository, Arc<Mutex<Option<Source>>>) {
        let mut repository = MockSourceRepository::new();
        let stored = Arc::new(Mutex::new(None::<Source>));
        let kept = stored.clone();
        repository.expect_insert().returning(move |source| {
            *kept.lock().unwrap() = Some(source.clone());
            Box::pin(async { Ok(()) })
        });
        let kept = stored.clone();
        repository.expect_update().returning(move |source| {
            *kept.lock().unwrap() = Some(source.clone());
            Box::pin(async { Ok(()) })
        });
        let read = stored.clone();
        repository.expect_find().returning(move |_, _| {
            let source = read.lock().unwrap().clone();
            Box::pin(async move {
                Ok(source.map(|source| SourceListing {
                    source,
                    events_last_24h: 0,
                    last_event_at: None,
                }))
            })
        });
        (repository, stored)
    }

    #[tokio::test]
    async fn a_source_is_created_with_only_the_hash_of_its_token() {
        let (repository, stored) = storing();
        let authorizer = allow();
        let organization_id = OrganizationId::new();

        let issued = SourceService::new(repository, &authorizer)
            .create(
                human(organization_id),
                command("chatwoot"),
                &mut MockCrewRepository::new(),
                &mut MockProjectRepository::new(),
            )
            .await
            .unwrap();

        let source = stored.lock().unwrap().clone().unwrap();
        assert_eq!(source.name, "Chatwoot");
        assert_eq!(source.webhook_token_hash, issued.token.hash());
        assert_ne!(source.webhook_token_hash.as_str(), issued.token.expose());
        assert_eq!(source.created_by, "user-1");
        assert!(!source.auto_run && !source.allow_writes);
        assert_eq!(issued.listing.source, source);
    }

    #[tokio::test]
    async fn an_unknown_or_archived_agent_and_bad_fields_store_nothing() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let mut archived = agent(organization_id, "old");
        archived.archived = true;
        let mut crew = MockCrewRepository::new();
        crew.expect_find().returning(move |_, _| {
            let agent = archived.clone();
            Box::pin(async move { Ok(Some(agent)) })
        });
        let mut with_agent = command("chatwoot");
        with_agent.default_agent_id = Some(AgentId::new());
        let mut bad_kind = command("chatwoot");
        bad_kind.kind = "rss".into();
        let mut bad_config = command("chatwoot");
        bad_config.config = Some(json!("x"));
        let mut bad_slug = command("Not Kebab");
        bad_slug.name = "x".into();
        for invalid in [with_agent, bad_kind, bad_config, bad_slug] {
            let error = SourceService::new(MockSourceRepository::new(), &authorizer)
                .create(
                    human(organization_id),
                    invalid,
                    &mut crew,
                    &mut MockProjectRepository::new(),
                )
                .await
                .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)), "{error}");
        }
    }

    #[tokio::test]
    async fn rotating_replaces_the_hash() {
        let organization_id = OrganizationId::new();
        let (repository, stored) = storing();
        let original = source(organization_id, "chatwoot");
        *stored.lock().unwrap() = Some(original.clone());
        let authorizer = allow();

        let issued = SourceService::new(repository, &authorizer)
            .rotate_token(human(organization_id), original.id)
            .await
            .unwrap();

        let saved = stored.lock().unwrap().clone().unwrap();
        assert_ne!(saved.webhook_token_hash, original.webhook_token_hash);
        assert_eq!(saved.webhook_token_hash, issued.token.hash());
    }

    #[tokio::test]
    async fn an_update_clears_the_agent_and_an_empty_one_is_refused() {
        let organization_id = OrganizationId::new();
        let (repository, stored) = storing();
        let mut original = source(organization_id, "chatwoot");
        original.default_agent_id = Some(AgentId::new());
        *stored.lock().unwrap() = Some(original.clone());
        let authorizer = allow();
        let updated = SourceService::new(repository, &authorizer)
            .update(
                human(organization_id),
                original.id,
                UpdateSource {
                    default_agent_id: Change::Clear,
                    enabled: Some(false),
                    ..UpdateSource::default()
                },
                &mut MockCrewRepository::new(),
                &mut MockProjectRepository::new(),
            )
            .await
            .unwrap();
        assert_eq!(updated.source.default_agent_id, None);
        assert!(!updated.source.enabled);

        let (repository, stored) = storing();
        *stored.lock().unwrap() = Some(original.clone());
        let error = SourceService::new(repository, &authorizer)
            .update(
                human(organization_id),
                original.id,
                UpdateSource::default(),
                &mut MockCrewRepository::new(),
                &mut MockProjectRepository::new(),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn a_token_reaches_only_an_enabled_active_source() {
        let organization_id = OrganizationId::new();
        let mut disabled = source(organization_id, "chatwoot");
        disabled.enabled = false;
        let mut repository = MockSourceRepository::new();
        repository.expect_find_by_token().returning(move |_| {
            let source = disabled.clone();
            Box::pin(async move { Ok(Some(source)) })
        });
        let authorizer = allow();
        let token = WebhookToken::generate().unwrap();
        let error = SourceService::new(repository, &authorizer)
            .by_token(token.expose())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::NotFound));

        let mut repository = MockSourceRepository::new();
        repository.expect_find_by_token().never();
        let error = SourceService::new(repository, &authorizer)
            .by_token("not-a-token")
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::NotFound));
    }

    #[tokio::test]
    async fn an_event_is_stored_received_with_its_summary_and_announced() {
        let organization_id = OrganizationId::new();
        let source = source(organization_id, "chatwoot");
        let mut events = MockSourceEventRepository::new();
        events
            .expect_count_since()
            .returning(|_, _| Box::pin(async { Ok(0) }));
        events
            .expect_insert()
            .times(1)
            .withf(|event| {
                event.status == SourceEventStatus::Received && event.summary == "Pièce reçue"
            })
            .returning(|_| Box::pin(async { Ok(()) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();

        let event = SourceEventService::new(events, &emitter, &authorizer)
            .receive(&source, json!({"title": "Pièce reçue"}), Utc::now())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(event.source_id, source.id);
        assert_eq!(*emitter.0.lock().unwrap(), ["source.event_received"]);
    }

    #[tokio::test]
    async fn a_source_over_its_rate_stores_nothing() {
        let source = source(OrganizationId::new(), "chatwoot");
        let mut events = MockSourceEventRepository::new();
        events
            .expect_count_since()
            .returning(|_, _| Box::pin(async { Ok(EVENTS_PER_MINUTE) }));
        events.expect_insert().never();
        let emitter = RecordingEmitter::default();
        let authorizer = allow();
        let refused = SourceEventService::new(events, &emitter, &authorizer)
            .receive(&source, json!({}), Utc::now())
            .await
            .unwrap();
        assert_eq!(refused, Err(HookRefusal::RateLimited));
        assert!(emitter.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_outcome_is_saved_and_announced() {
        let source = source(OrganizationId::new(), "chatwoot");
        let event = SourceEvent {
            id: SourceEventId::new(),
            organization_id: source.organization_id,
            source_id: source.id,
            received_at: Utc::now(),
            status: SourceEventStatus::Received,
            payload: json!({}),
            summary: String::new(),
            card_id: None,
            run_id: None,
            error: None,
        };
        let mut events = MockSourceEventRepository::new();
        events
            .expect_save_outcome()
            .times(1)
            .withf(|event| {
                event.status == SourceEventStatus::CardCreated
                    && event.error.as_deref().map(|e| e.chars().count()) == Some(ERROR_MAX_CHARS)
            })
            .returning(|_| Box::pin(async { Ok(()) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();
        let card_id = CardId::new();
        let recorded = SourceEventService::new(events, &emitter, &authorizer)
            .record(
                event,
                EventOutcome {
                    status: SourceEventStatus::CardCreated,
                    card_id: Some(card_id),
                    run_id: None,
                    note: Some("x".repeat(5_000)),
                },
            )
            .await
            .unwrap();
        assert_eq!(recorded.card_id, Some(card_id));
        assert_eq!(*emitter.0.lock().unwrap(), ["source.event_processed"]);
    }

    #[tokio::test]
    async fn a_denied_member_changes_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "humans only".into(),
                })
            })
        });
        let error = SourceService::new(MockSourceRepository::new(), &authorizer)
            .create(
                human(OrganizationId::new()),
                command("chatwoot"),
                &mut MockCrewRepository::new(),
                &mut MockProjectRepository::new(),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
