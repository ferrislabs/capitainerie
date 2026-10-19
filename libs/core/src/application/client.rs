use capitainerie_common::{CardId, ClientId, CoreError, OrganizationId, ProjectId, RunId};
use capitainerie_events::card::{CardAuthor, CardUpdated};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    authorization::Subject,
    card::{
        Card,
        link::{CardLinkService, LinkCard},
        ports::CardRepository,
    },
    client::{
        Client, ClientDetail, ClientListing, ListFilter,
        catalog::{self, Catalog},
        links::Links,
        service::{ClientService, CreateClient, UpdateClient},
    },
    event::ports::EventEmitter,
    project::{
        Project, ProjectDetail, ProjectFilter, ProjectListing,
        service::{CreateProject, ProjectService, UpdateProject},
    },
};

#[derive(Debug, Clone)]
pub struct RunCardLinks {
    pub span: Option<(u32, u32)>,
    pub links: Links,
}

impl CapitainerieUseCase {
    #[transactional(client, project, authz)]
    pub async fn list_clients(
        &self,
        subject: Subject,
        filter: ListFilter,
    ) -> Result<Vec<ClientListing>, CoreError> {
        ClientService::new(client_repository, project_repository, authz)
            .list(subject, filter)
            .await
    }

    #[transactional(client, project, authz)]
    pub async fn get_client(
        &self,
        subject: Subject,
        id: ClientId,
    ) -> Result<ClientDetail, CoreError> {
        ClientService::new(client_repository, project_repository, authz)
            .get(subject, id)
            .await
    }

    #[transactional(client, project, authz)]
    pub async fn create_client(
        &self,
        subject: Subject,
        command: CreateClient,
    ) -> Result<Client, CoreError> {
        ClientService::new(client_repository, project_repository, authz)
            .create(subject, command)
            .await
    }

    #[transactional(client, project, authz)]
    pub async fn update_client(
        &self,
        subject: Subject,
        id: ClientId,
        command: UpdateClient,
    ) -> Result<Client, CoreError> {
        ClientService::new(client_repository, project_repository, authz)
            .update(subject, id, command)
            .await
    }

    #[transactional(client, project, authz)]
    pub async fn list_projects(
        &self,
        subject: Subject,
        filter: ProjectFilter,
    ) -> Result<Vec<ProjectListing>, CoreError> {
        ProjectService::new(project_repository, client_repository, authz)
            .list(subject, filter)
            .await
    }

    #[transactional(client, project, authz)]
    pub async fn get_project(
        &self,
        subject: Subject,
        id: ProjectId,
    ) -> Result<ProjectDetail, CoreError> {
        ProjectService::new(project_repository, client_repository, authz)
            .get(subject, id)
            .await
    }

    #[transactional(client, project, authz)]
    pub async fn create_project(
        &self,
        subject: Subject,
        command: CreateProject,
    ) -> Result<Project, CoreError> {
        ProjectService::new(project_repository, client_repository, authz)
            .create(subject, command)
            .await
    }

    #[transactional(client, project, authz)]
    pub async fn update_project(
        &self,
        subject: Subject,
        id: ProjectId,
        command: UpdateProject,
    ) -> Result<Project, CoreError> {
        ProjectService::new(project_repository, client_repository, authz)
            .update(subject, id, command)
            .await
    }

    #[transactional(card, client, project, emitter, authz)]
    pub async fn link_card(
        &self,
        subject: Subject,
        id: CardId,
        command: LinkCard,
    ) -> Result<Card, CoreError> {
        CardLinkService::new(
            card_repository,
            client_repository,
            project_repository,
            emitter,
            authz,
        )
        .link(subject, id, command)
        .await
    }

    #[transactional(client, project)]
    pub async fn detection_catalog(
        &self,
        organization_id: OrganizationId,
    ) -> Result<Catalog, CoreError> {
        let mut client_repository = client_repository;
        let mut project_repository = project_repository;
        catalog::load(
            &mut client_repository,
            &mut project_repository,
            organization_id,
        )
        .await
    }

    #[transactional(card, emitter)]
    pub async fn link_run_cards(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
        initiated_by: &str,
        agent: &str,
        batches: Vec<RunCardLinks>,
    ) -> Result<usize, CoreError> {
        let mut card_repository = card_repository;
        let mut linked = 0;
        for batch in batches.into_iter().filter(|batch| !batch.links.is_empty()) {
            let cards = card_repository
                .link_run_cards(organization_id, run_id, batch.span, &batch.links)
                .await?;
            linked += cards.len();
            for (card_id, version) in cards {
                emitter.emit(&CardUpdated {
                    card_id: card_id.0,
                    organization_id: organization_id.0,
                    version,
                    run_id: Some(run_id.0),
                    author: CardAuthor {
                        subject: initiated_by.to_owned(),
                        actor: Some(agent.to_owned()),
                    },
                })?;
            }
        }
        Ok(linked)
    }
}
