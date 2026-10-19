use capitainerie_common::{ClientId, CoreError};
use chrono::Utc;

use super::{
    Client, ClientDetail, ClientListing, ColumnCount, ListFilter, ports::ClientRepository,
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    card::bounded_text,
    label::{Aliases, NOTES_MAX_CHARS, Name},
    project::ports::ProjectRepository,
};

#[derive(Debug, Clone)]
pub struct CreateClient {
    pub name: String,
    pub aliases: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateClient {
    pub name: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub notes: Option<String>,
    pub archived: Option<bool>,
}

pub struct ClientService<'a, C, P, A> {
    clients: C,
    projects: P,
    authorizer: &'a A,
}

impl<'a, C, P, A> ClientService<'a, C, P, A>
where
    C: ClientRepository,
    P: ProjectRepository,
    A: Authorizer,
{
    pub fn new(clients: C, projects: P, authorizer: &'a A) -> Self {
        Self {
            clients,
            projects,
            authorizer,
        }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        filter: ListFilter,
    ) -> Result<Vec<ClientListing>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ClientRead, None).await?;
        self.clients.list(organization_id, &filter).await
    }

    pub async fn get(mut self, subject: Subject, id: ClientId) -> Result<ClientDetail, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ClientRead, Some(id)).await?;
        let client = self
            .clients
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        let projects = self.projects.of_client(organization_id, id).await?;
        let counted = self.clients.card_counts(organization_id, id).await?;
        Ok(ClientDetail {
            client,
            projects,
            cards: ColumnCount::complete(&counted),
        })
    }

    pub async fn create(
        mut self,
        subject: Subject,
        command: CreateClient,
    ) -> Result<Client, CoreError> {
        self.require(subject.clone(), Action::ClientManage, None)
            .await?;
        let now = Utc::now();
        let client = Client {
            id: ClientId::new(),
            organization_id: subject.organization_id,
            name: Name::try_from(command.name)?,
            aliases: Aliases::try_from(command.aliases)?,
            notes: bounded_text(command.notes, NOTES_MAX_CHARS, "notes", true)?,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        self.clients.insert(&client).await?;
        Ok(client)
    }

    pub async fn update(
        mut self,
        subject: Subject,
        id: ClientId,
        command: UpdateClient,
    ) -> Result<Client, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ClientManage, Some(id))
            .await?;
        let mut client = self
            .clients
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if command.name.is_none()
            && command.aliases.is_none()
            && command.notes.is_none()
            && command.archived.is_none()
        {
            return Err(CoreError::Invalid(
                "nothing to update: give a name, aliases, notes or archived".into(),
            ));
        }
        let now = Utc::now();
        if let Some(name) = command.name {
            client.name = Name::try_from(name)?;
        }
        if let Some(aliases) = command.aliases {
            client.aliases = Aliases::try_from(aliases)?;
        }
        if let Some(notes) = command.notes {
            client.notes = bounded_text(notes, NOTES_MAX_CHARS, "notes", true)?;
        }
        match command.archived {
            Some(true) if client.archived_at.is_none() => client.archived_at = Some(now),
            Some(false) => client.archived_at = None,
            _ => {}
        }
        client.updated_at = now;
        self.clients.update(&client).await?;
        Ok(client)
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        id: Option<ClientId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, action, ResourceKind::Client, id.map(|id| id.0)),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        client::{links::tests::client, ports::MockClientRepository},
        project::ports::MockProjectRepository,
    };

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

    fn holding(client: Client) -> MockClientRepository {
        let mut clients = MockClientRepository::new();
        clients.expect_find().returning(move |_, _| {
            let client = client.clone();
            Box::pin(async move { Ok(Some(client)) })
        });
        clients
            .expect_update()
            .returning(|_| Box::pin(async { Ok(()) }));
        clients
    }

    #[tokio::test]
    async fn a_client_is_created_with_a_clean_name_and_aliases() {
        let mut clients = MockClientRepository::new();
        clients
            .expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = allow();
        let organization_id = OrganizationId::new();

        let created = ClientService::new(clients, MockProjectRepository::new(), &authorizer)
            .create(
                human(organization_id),
                CreateClient {
                    name: "  Acme ".into(),
                    aliases: vec!["ACME SA".into(), "acme sa".into()],
                    notes: String::new(),
                },
            )
            .await
            .unwrap();

        assert_eq!(created.name.as_str(), "Acme");
        assert_eq!(created.aliases.as_slice(), ["ACME SA"]);
        assert_eq!(created.organization_id, organization_id);
        assert_eq!(created.created_by, "user-1");
    }

    #[tokio::test]
    async fn invalid_input_stores_nothing() {
        let authorizer = allow();
        for (name, aliases, notes) in [
            ("Dupont, Jean", vec![], ""),
            ("Acme", vec!["a,b".to_owned()], ""),
            ("", vec![], ""),
            ("Acme", vec![], &"x".repeat(NOTES_MAX_CHARS + 1)[..]),
        ] {
            let error = ClientService::new(
                MockClientRepository::new(),
                MockProjectRepository::new(),
                &authorizer,
            )
            .create(
                human(OrganizationId::new()),
                CreateClient {
                    name: name.into(),
                    aliases,
                    notes: notes.into(),
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)), "{name:?}");
        }
    }

    #[tokio::test]
    async fn a_denied_manager_changes_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "humans only".into(),
                })
            })
        });
        let error = ClientService::new(
            MockClientRepository::new(),
            MockProjectRepository::new(),
            &authorizer,
        )
        .create(
            human(OrganizationId::new()),
            CreateClient {
                name: "Acme".into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }

    #[tokio::test]
    async fn archiving_and_unarchiving_set_and_clear_the_date() {
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let authorizer = allow();

        let archived = ClientService::new(
            holding(acme.clone()),
            MockProjectRepository::new(),
            &authorizer,
        )
        .update(
            human(organization_id),
            acme.id,
            UpdateClient {
                archived: Some(true),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();
        assert!(archived.is_archived());

        let restored = ClientService::new(
            holding(archived.clone()),
            MockProjectRepository::new(),
            &authorizer,
        )
        .update(
            human(organization_id),
            acme.id,
            UpdateClient {
                archived: Some(false),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();
        assert!(!restored.is_archived());
    }

    #[tokio::test]
    async fn an_empty_update_and_an_unknown_client_are_refused() {
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let authorizer = allow();
        let error = ClientService::new(
            holding(acme.clone()),
            MockProjectRepository::new(),
            &authorizer,
        )
        .update(human(organization_id), acme.id, UpdateClient::default())
        .await
        .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));

        let mut clients = MockClientRepository::new();
        clients
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        let error = ClientService::new(clients, MockProjectRepository::new(), &authorizer)
            .get(human(organization_id), ClientId::new())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::NotFound));
    }
}
