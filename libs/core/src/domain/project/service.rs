use capitainerie_common::{ClientId, CoreError, ProjectId};
use capitainerie_events::card::{CardAuthor, CardUpdated};
use chrono::Utc;

use super::{Project, ProjectDetail, ProjectFilter, ProjectListing, ports::ProjectRepository};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    card::bounded_text,
    client::{
        ColumnCount,
        links::{Change, active_client},
        ports::ClientRepository,
    },
    event::ports::EventEmitter,
    execution::RepositoryRef,
    label::{Aliases, NOTES_MAX_CHARS, Name},
};

#[derive(Debug, Clone)]
pub struct CreateProject {
    pub client_id: Option<ClientId>,
    pub name: String,
    pub aliases: Vec<String>,
    pub repository: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateProject {
    pub client_id: Change<ClientId>,
    pub name: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub repository: Change<String>,
    pub description: Option<String>,
    pub archived: Option<bool>,
}

pub struct ProjectService<'a, P, C, E, A> {
    projects: P,
    clients: C,
    emitter: &'a E,
    authorizer: &'a A,
}

impl<'a, P, C, E, A> ProjectService<'a, P, C, E, A>
where
    P: ProjectRepository,
    C: ClientRepository,
    E: EventEmitter,
    A: Authorizer,
{
    pub fn new(projects: P, clients: C, emitter: &'a E, authorizer: &'a A) -> Self {
        Self {
            projects,
            clients,
            emitter,
            authorizer,
        }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        filter: ProjectFilter,
    ) -> Result<Vec<ProjectListing>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ClientRead, None).await?;
        self.projects.list(organization_id, &filter).await
    }

    pub async fn get(
        mut self,
        subject: Subject,
        id: ProjectId,
    ) -> Result<ProjectDetail, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ClientRead, Some(id)).await?;
        let project = self
            .projects
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        let counted = self.projects.card_counts(organization_id, id).await?;
        Ok(ProjectDetail {
            project,
            cards: ColumnCount::complete(&counted),
        })
    }

    pub async fn create(
        mut self,
        subject: Subject,
        command: CreateProject,
    ) -> Result<Project, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject.clone(), Action::ClientManage, None)
            .await?;
        let client = match command.client_id {
            Some(id) => Some(active_client(&mut self.clients, organization_id, id).await?),
            None => None,
        };
        let now = Utc::now();
        let project = Project {
            id: ProjectId::new(),
            organization_id,
            client,
            name: Name::try_from(command.name)?,
            aliases: Aliases::try_from(command.aliases)?,
            repository: command
                .repository
                .map(|value| value.trim().parse::<RepositoryRef>())
                .transpose()?,
            description: bounded_text(command.description, NOTES_MAX_CHARS, "description", true)?,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        self.projects.insert(&project).await?;
        Ok(project)
    }

    pub async fn update(
        mut self,
        subject: Subject,
        id: ProjectId,
        command: UpdateProject,
    ) -> Result<Project, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject.clone(), Action::ClientManage, Some(id))
            .await?;
        let mut project = self
            .projects
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if command.client_id == Change::Keep
            && command.name.is_none()
            && command.aliases.is_none()
            && command.repository == Change::Keep
            && command.description.is_none()
            && command.archived.is_none()
        {
            return Err(CoreError::Invalid(
                "nothing to update: give a field to change".into(),
            ));
        }
        let now = Utc::now();
        match command.client_id {
            Change::Keep => {}
            Change::Clear => project.client = None,
            Change::Set(client_id) if project.client_id() == Some(client_id) => {}
            Change::Set(client_id) => {
                project.client =
                    Some(active_client(&mut self.clients, organization_id, client_id).await?);
            }
        }
        if let Some(name) = command.name {
            project.name = Name::try_from(name)?;
        }
        if let Some(aliases) = command.aliases {
            project.aliases = Aliases::try_from(aliases)?;
        }
        match command.repository {
            Change::Keep => {}
            Change::Clear => project.repository = None,
            Change::Set(value) => project.repository = Some(value.trim().parse()?),
        }
        if let Some(description) = command.description {
            project.description = bounded_text(description, NOTES_MAX_CHARS, "description", true)?;
        }
        match command.archived {
            Some(true) if project.archived_at.is_none() => project.archived_at = Some(now),
            Some(false) => project.archived_at = None,
            _ => {}
        }
        project.updated_at = now;
        let carried = self.projects.update(&project).await?;
        for (card_id, version) in carried {
            self.emitter.emit(&CardUpdated {
                card_id: card_id.0,
                organization_id: organization_id.0,
                version,
                run_id: None,
                author: CardAuthor {
                    subject: subject.id.clone(),
                    actor: subject.actor.clone(),
                },
            })?;
        }
        Ok(project)
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        id: Option<ProjectId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, action, ResourceKind::Project, id.map(|id| id.0)),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use capitainerie_common::{CardId, OrganizationId};
    use capitainerie_events::DomainEvent;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        client::{
            Client,
            links::tests::{client, project},
            ports::MockClientRepository,
        },
        project::ports::MockProjectRepository,
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

    fn clients_knowing(known: Client) -> MockClientRepository {
        let mut clients = MockClientRepository::new();
        clients.expect_find().returning(move |_, id| {
            let found = (id == known.id).then(|| known.clone());
            Box::pin(async move { Ok(found) })
        });
        clients
    }

    fn creation(client_id: Option<ClientId>) -> CreateProject {
        CreateProject {
            client_id,
            name: "Refonte".into(),
            aliases: vec![],
            repository: Some(" acme/web ".into()),
            description: String::new(),
        }
    }

    #[tokio::test]
    async fn a_project_is_created_under_an_existing_client_with_a_valid_repository() {
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let mut projects = MockProjectRepository::new();
        projects
            .expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = allow();
        let emitter = RecordingEmitter::default();

        let created = ProjectService::new(
            projects,
            clients_knowing(acme.clone()),
            &emitter,
            &authorizer,
        )
        .create(human(organization_id), creation(Some(acme.id)))
        .await
        .unwrap();

        assert_eq!(created.client_id(), Some(acme.id));
        assert_eq!(created.repository.unwrap().to_string(), "acme/web");
    }

    #[tokio::test]
    async fn a_project_cannot_point_to_an_unknown_or_archived_client() {
        let organization_id = OrganizationId::new();
        let mut archived = client(organization_id, "Vieux");
        archived.archived_at = Some(Utc::now());
        let authorizer = allow();
        let emitter = RecordingEmitter::default();
        for id in [ClientId::new(), archived.id] {
            let error = ProjectService::new(
                MockProjectRepository::new(),
                clients_knowing(archived.clone()),
                &emitter,
                &authorizer,
            )
            .create(human(organization_id), creation(Some(id)))
            .await
            .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)), "{error:?}");
        }
    }

    #[tokio::test]
    async fn a_bad_repository_is_invalid() {
        let authorizer = allow();
        let emitter = RecordingEmitter::default();
        let mut command = creation(None);
        command.repository = Some("../etc".into());
        let error = ProjectService::new(
            MockProjectRepository::new(),
            MockClientRepository::new(),
            &emitter,
            &authorizer,
        )
        .create(human(OrganizationId::new()), command)
        .await
        .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn changing_the_client_and_clearing_the_repository_are_explicit() {
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let globex = client(organization_id, "Globex");
        let mut site = project(organization_id, "Site", Some(&acme));
        site.repository = Some("acme/web".parse().unwrap());
        let mut projects = MockProjectRepository::new();
        let held = site.clone();
        projects.expect_find().returning(move |_, _| {
            let held = held.clone();
            Box::pin(async move { Ok(Some(held)) })
        });
        projects
            .expect_update()
            .returning(|_| Box::pin(async { Ok(vec![(CardId::new(), 4)]) }));
        let authorizer = allow();
        let emitter = RecordingEmitter::default();

        let moved = ProjectService::new(
            projects,
            clients_knowing(globex.clone()),
            &emitter,
            &authorizer,
        )
        .update(
            human(organization_id),
            site.id,
            UpdateProject {
                client_id: Change::Set(globex.id),
                repository: Change::Clear,
                ..UpdateProject::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(moved.client_id(), Some(globex.id));
        assert_eq!(moved.repository, None);
        assert_eq!(*emitter.0.lock().unwrap(), [CardUpdated::KIND]);
    }
}
