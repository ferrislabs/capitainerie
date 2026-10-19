use capitainerie_common::{CardId, ClientId, CoreError, ProjectId};
use capitainerie_events::card::{CardAuthor, CardUpdated};
use chrono::Utc;

use super::{Author, Card, ports::CardRepository};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    client::{
        ClientRef,
        links::{Change, Links, active_client, active_project},
        ports::ClientRepository,
    },
    event::ports::EventEmitter,
    project::{Project, ports::ProjectRepository},
};

#[derive(Debug, Clone)]
pub struct LinkCard {
    pub expected_version: i32,
    pub client: Change<ClientId>,
    pub project: Change<ProjectId>,
}

pub struct CardLinkService<'a, C, K, P, E, A> {
    cards: C,
    clients: K,
    projects: P,
    emitter: &'a E,
    authorizer: &'a A,
}

impl<'a, C, K, P, E, A> CardLinkService<'a, C, K, P, E, A>
where
    C: CardRepository,
    K: ClientRepository,
    P: ProjectRepository,
    E: EventEmitter,
    A: Authorizer,
{
    pub fn new(cards: C, clients: K, projects: P, emitter: &'a E, authorizer: &'a A) -> Self {
        Self {
            cards,
            clients,
            projects,
            emitter,
            authorizer,
        }
    }

    pub async fn link(
        mut self,
        subject: Subject,
        id: CardId,
        command: LinkCard,
    ) -> Result<Card, CoreError> {
        let organization_id = subject.organization_id;
        let mut card = self
            .cards
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        require(
            self.authorizer,
            AccessRequest::new(
                subject.clone(),
                Action::CardLink,
                ResourceKind::Card,
                Some(id.0),
            ),
        )
        .await?;
        card.ensure_version(command.expected_version)?;
        if command.client == Change::Keep && command.project == Change::Keep {
            return Err(CoreError::Invalid(
                "nothing to update: give a client_id or a project_id".into(),
            ));
        }

        let links = self.target(&card, &command).await?;
        if links == card.links {
            return Ok(card);
        }
        card.links = links;
        card.touch(Utc::now());
        self.cards.update(&card).await?;
        self.emitter.emit(&CardUpdated {
            card_id: card.id.0,
            organization_id: card.organization_id.0,
            version: card.version,
            run_id: None,
            author: CardAuthor::from(&Author {
                subject: subject.id,
                actor: subject.actor,
            }),
        })?;
        Ok(card)
    }

    async fn target(&mut self, card: &Card, command: &LinkCard) -> Result<Links, CoreError> {
        let organization_id = card.organization_id;
        let project: Option<Project> = match command.project {
            Change::Keep => match card.links.project() {
                Some(current) => Some(
                    self.projects
                        .find(organization_id, current.id)
                        .await?
                        .ok_or_else(|| {
                            CoreError::Internal(format!("project {} vanished", current.id))
                        })?,
                ),
                None => None,
            },
            Change::Clear => None,
            Change::Set(id) if card.links.project_id() == Some(id) => self
                .projects
                .find(organization_id, id)
                .await?
                .ok_or_else(|| CoreError::Invalid(format!("unknown project {id}")))
                .map(Some)?,
            Change::Set(id) => Some(active_project(&mut self.projects, organization_id, id).await?),
        };
        let project_changed = command.project != Change::Keep;
        let client: Option<ClientRef> = match command.client {
            Change::Set(id) if card.links.client_id() == Some(id) => card.links.client().cloned(),
            Change::Set(id) => Some(active_client(&mut self.clients, organization_id, id).await?),
            Change::Clear => {
                if project
                    .as_ref()
                    .is_some_and(|project| project.client.is_some())
                {
                    return Err(CoreError::Invalid(
                        "the project belongs to a client: clear the project to clear the client"
                            .into(),
                    ));
                }
                None
            }
            Change::Keep if project_changed && project.is_some() => None,
            Change::Keep => card.links.client().cloned(),
        };
        Links::of(client, project.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use capitainerie_common::OrganizationId;
    use capitainerie_events::DomainEvent;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        card::{CardTitle, Column, ports::MockCardRepository},
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

    fn human(organization_id: capitainerie_common::OrganizationId) -> Subject {
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

    fn stored(organization_id: OrganizationId, links: Links) -> Card {
        Card {
            id: CardId::new(),
            organization_id,
            title: CardTitle::try_from("Migrer la CI".to_string()).unwrap(),
            description: String::new(),
            column: Column::Todo,
            provenance: None,
            created_by_run: None,
            created_by: Author {
                subject: "user-1".into(),
                actor: None,
            },
            version: 3,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            repository_hint: None,
            links,
        }
    }

    fn cards_holding(card: Card, updates: bool) -> MockCardRepository {
        let mut cards = MockCardRepository::new();
        cards.expect_find().returning(move |_, _| {
            let card = card.clone();
            Box::pin(async move { Ok(Some(card)) })
        });
        if updates {
            cards
                .expect_update()
                .times(1)
                .returning(|_| Box::pin(async { Ok(()) }));
        } else {
            cards.expect_update().never();
        }
        cards
    }

    fn knowing(
        clients: Vec<Client>,
        projects: Vec<Project>,
    ) -> (MockClientRepository, MockProjectRepository) {
        let mut client_repository = MockClientRepository::new();
        client_repository.expect_find().returning(move |_, id| {
            let found = clients.iter().find(|client| client.id == id).cloned();
            Box::pin(async move { Ok(found) })
        });
        let mut project_repository = MockProjectRepository::new();
        project_repository.expect_find().returning(move |_, id| {
            let found = projects.iter().find(|project| project.id == id).cloned();
            Box::pin(async move { Ok(found) })
        });
        (client_repository, project_repository)
    }

    async fn link(
        card: Card,
        updates: bool,
        clients: Vec<Client>,
        projects: Vec<Project>,
        command: LinkCard,
    ) -> (Result<Card, CoreError>, Vec<String>) {
        let emitter = RecordingEmitter::default();
        let authorizer = allow();
        let (client_repository, project_repository) = knowing(clients, projects);
        let organization_id = card.organization_id;
        let id = card.id;
        let result = CardLinkService::new(
            cards_holding(card, updates),
            client_repository,
            project_repository,
            &emitter,
            &authorizer,
        )
        .link(human(organization_id), id, command)
        .await;
        let events = emitter.0.lock().unwrap().clone();
        (result, events)
    }

    fn command(client: Change<ClientId>, project: Change<ProjectId>) -> LinkCard {
        LinkCard {
            expected_version: 3,
            client,
            project,
        }
    }

    #[tokio::test]
    async fn a_client_is_set_and_the_card_changes_version() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let (card, events) = link(
            stored(org, Links::default()),
            true,
            vec![acme.clone()],
            vec![],
            command(Change::Set(acme.id), Change::Keep),
        )
        .await;
        let card = card.unwrap();
        assert_eq!(card.links.client_id(), Some(acme.id));
        assert_eq!(card.version, 4);
        assert_eq!(events, [CardUpdated::KIND]);
    }

    #[tokio::test]
    async fn a_project_brings_its_client_to_the_card() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let site = project(org, "Site", Some(&acme));
        let (card, _) = link(
            stored(org, Links::default()),
            true,
            vec![acme.clone()],
            vec![site.clone()],
            command(Change::Keep, Change::Set(site.id)),
        )
        .await;
        let card = card.unwrap();
        assert_eq!(card.links.project_id(), Some(site.id));
        assert_eq!(card.links.client_id(), Some(acme.id));
    }

    #[tokio::test]
    async fn moving_to_another_clients_project_replaces_the_cards_client() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let globex = client(org, "Globex");
        let old = project(org, "Site", Some(&acme));
        let new = project(org, "App", Some(&globex));
        let current = Links::of(None, Some(&old)).unwrap();
        let (card, _) = link(
            stored(org, current),
            true,
            vec![acme, globex.clone()],
            vec![old, new.clone()],
            command(Change::Keep, Change::Set(new.id)),
        )
        .await;
        let card = card.unwrap();
        assert_eq!(card.links.project_id(), Some(new.id));
        assert_eq!(card.links.client_id(), Some(globex.id));
    }

    #[tokio::test]
    async fn a_client_that_is_not_the_projects_is_invalid_and_nothing_is_written() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let globex = client(org, "Globex");
        let site = project(org, "Site", Some(&acme));
        let (result, events) = link(
            stored(org, Links::default()),
            false,
            vec![acme, globex.clone()],
            vec![site.clone()],
            command(Change::Set(globex.id), Change::Set(site.id)),
        )
        .await;
        assert!(matches!(result, Err(CoreError::Invalid(_))));
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn changing_the_client_of_a_card_whose_project_has_one_is_invalid() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let globex = client(org, "Globex");
        let site = project(org, "Site", Some(&acme));
        let current = Links::of(None, Some(&site)).unwrap();
        for change in [Change::Set(globex.id), Change::Clear] {
            let (result, _) = link(
                stored(org, current.clone()),
                false,
                vec![acme.clone(), globex.clone()],
                vec![site.clone()],
                command(change, Change::Keep),
            )
            .await;
            assert!(matches!(result, Err(CoreError::Invalid(_))), "{change:?}");
        }
    }

    #[tokio::test]
    async fn clearing_the_project_keeps_the_client_and_clearing_both_empties_the_card() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let site = project(org, "Site", Some(&acme));
        let current = Links::of(None, Some(&site)).unwrap();

        let (kept, _) = link(
            stored(org, current.clone()),
            true,
            vec![acme.clone()],
            vec![site.clone()],
            command(Change::Keep, Change::Clear),
        )
        .await;
        let kept = kept.unwrap();
        assert_eq!(kept.links.project(), None);
        assert_eq!(kept.links.client_id(), Some(acme.id));

        let (emptied, _) = link(
            stored(org, current),
            true,
            vec![acme],
            vec![site],
            command(Change::Clear, Change::Clear),
        )
        .await;
        assert!(emptied.unwrap().links.is_empty());
    }

    #[tokio::test]
    async fn an_unknown_client_or_project_is_invalid_not_a_not_found() {
        let org = OrganizationId::new();
        for change in [
            command(Change::Set(ClientId::new()), Change::Keep),
            command(Change::Keep, Change::Set(ProjectId::new())),
        ] {
            let (result, _) =
                link(stored(org, Links::default()), false, vec![], vec![], change).await;
            assert!(matches!(result, Err(CoreError::Invalid(_))));
        }
    }

    #[tokio::test]
    async fn an_archived_client_cannot_be_newly_chosen() {
        let org = OrganizationId::new();
        let mut old = client(org, "Vieux");
        old.archived_at = Some(Utc::now());
        let (result, _) = link(
            stored(org, Links::default()),
            false,
            vec![old.clone()],
            vec![],
            command(Change::Set(old.id), Change::Keep),
        )
        .await;
        assert!(matches!(result, Err(CoreError::Invalid(_))));
    }

    #[tokio::test]
    async fn a_stale_version_is_a_conflict_and_writes_nothing() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let mut stale = command(Change::Set(acme.id), Change::Keep);
        stale.expected_version = 2;
        let (result, events) = link(
            stored(org, Links::default()),
            false,
            vec![acme],
            vec![],
            stale,
        )
        .await;
        assert!(matches!(result, Err(CoreError::Conflict(_))));
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn an_empty_patch_is_invalid_and_an_identical_link_changes_nothing() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let (result, _) = link(
            stored(org, Links::default()),
            false,
            vec![],
            vec![],
            command(Change::Keep, Change::Keep),
        )
        .await;
        assert!(matches!(result, Err(CoreError::Invalid(_))));

        let current = Links::of(Some(acme.reference()), None).unwrap();
        let (same, events) = link(
            stored(org, current),
            false,
            vec![acme.clone()],
            vec![],
            command(Change::Set(acme.id), Change::Keep),
        )
        .await;
        assert_eq!(same.unwrap().version, 3);
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn the_policy_decides_before_anything_is_changed() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .withf(|request| request.action == Action::CardLink)
            .returning(|_| {
                Box::pin(async {
                    Ok(Decision::Deny {
                        reason: "humans only".into(),
                    })
                })
            });
        let emitter = RecordingEmitter::default();
        let card = stored(org, Links::default());
        let id = card.id;
        let (client_repository, project_repository) = knowing(vec![acme.clone()], vec![]);
        let error = CardLinkService::new(
            cards_holding(card, false),
            client_repository,
            project_repository,
            &emitter,
            &authorizer,
        )
        .link(
            Subject {
                actor: Some("capitainerie-agent-writer".into()),
                ..human(org)
            },
            id,
            command(Change::Set(acme.id), Change::Keep),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
