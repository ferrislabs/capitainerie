use capitainerie_common::{ClientId, CoreError, OrganizationId, ProjectId};

use super::{ClientRef, ports::ClientRepository};
use crate::domain::{
    execution::RepositoryRef,
    project::{Project, ProjectRef, ports::ProjectRepository},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Links {
    client: Option<ClientRef>,
    project: Option<ProjectRef>,
}

impl Links {
    pub fn client(&self) -> Option<&ClientRef> {
        self.client.as_ref()
    }

    pub fn project(&self) -> Option<&ProjectRef> {
        self.project.as_ref()
    }

    pub fn is_empty(&self) -> bool {
        self.client.is_none() && self.project.is_none()
    }

    pub fn client_id(&self) -> Option<ClientId> {
        self.client.as_ref().map(|client| client.id)
    }

    pub fn project_id(&self) -> Option<ProjectId> {
        self.project.as_ref().map(|project| project.id)
    }

    pub fn of(client: Option<ClientRef>, project: Option<&Project>) -> Result<Self, CoreError> {
        let Some(project) = project else {
            return Ok(Self {
                client,
                project: None,
            });
        };
        let client = match (client, &project.client) {
            (Some(chosen), Some(owner)) if chosen.id != owner.id => {
                return Err(CoreError::Invalid(format!(
                    "the project {} belongs to the client {}, not {}",
                    project.name.as_str(),
                    owner.name,
                    chosen.name
                )));
            }
            (Some(chosen), None) => {
                return Err(CoreError::Invalid(format!(
                    "the project {} has no client, so the client {} cannot go with it",
                    project.name.as_str(),
                    chosen.name
                )));
            }
            (_, owner) => owner.clone(),
        };
        Ok(Self {
            client,
            project: Some(project.reference()),
        })
    }

    pub fn stored(client: Option<ClientRef>, project: Option<ProjectRef>) -> Self {
        Self { client, project }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Change<T> {
    #[default]
    Keep,
    Clear,
    Set(T),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LinkChoice {
    pub client_id: Option<ClientId>,
    pub project_id: Option<ProjectId>,
}

impl LinkChoice {
    pub fn is_empty(&self) -> bool {
        self.client_id.is_none() && self.project_id.is_none()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chosen {
    pub links: Links,
    pub repository: Option<RepositoryRef>,
}

pub async fn resolve_choice<C: ClientRepository, P: ProjectRepository>(
    clients: &mut C,
    projects: &mut P,
    organization_id: OrganizationId,
    choice: LinkChoice,
) -> Result<Chosen, CoreError> {
    let project = match choice.project_id {
        Some(id) => Some(active_project(projects, organization_id, id).await?),
        None => None,
    };
    let client = match choice.client_id {
        Some(id) => Some(active_client(clients, organization_id, id).await?),
        None => None,
    };
    Ok(Chosen {
        repository: project
            .as_ref()
            .and_then(|project| project.repository.clone()),
        links: Links::of(client, project.as_ref())?,
    })
}

pub async fn active_client<C: ClientRepository>(
    clients: &mut C,
    organization_id: OrganizationId,
    id: ClientId,
) -> Result<ClientRef, CoreError> {
    let client = clients
        .find(organization_id, id)
        .await?
        .ok_or_else(|| CoreError::Invalid(format!("unknown client {id}")))?;
    if client.is_archived() {
        return Err(CoreError::Invalid(format!(
            "the client {} is archived",
            client.name.as_str()
        )));
    }
    Ok(client.reference())
}

pub async fn active_project<P: ProjectRepository>(
    projects: &mut P,
    organization_id: OrganizationId,
    id: ProjectId,
) -> Result<Project, CoreError> {
    let project = projects
        .find(organization_id, id)
        .await?
        .ok_or_else(|| CoreError::Invalid(format!("unknown project {id}")))?;
    if project.is_archived() {
        return Err(CoreError::Invalid(format!(
            "the project {} is archived",
            project.name.as_str()
        )));
    }
    Ok(project)
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::{
        client::{Client, ports::MockClientRepository},
        label::{Aliases, Name},
        project::ports::MockProjectRepository,
    };

    pub(crate) fn client(organization_id: OrganizationId, name: &str) -> Client {
        Client {
            id: ClientId::new(),
            organization_id,
            name: Name::try_from(name.to_owned()).unwrap(),
            aliases: Aliases::default(),
            notes: String::new(),
            created_by: "user-1".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
        }
    }

    pub(crate) fn project(
        organization_id: OrganizationId,
        name: &str,
        client: Option<&Client>,
    ) -> Project {
        Project {
            id: ProjectId::new(),
            organization_id,
            client: client.map(Client::reference),
            name: Name::try_from(name.to_owned()).unwrap(),
            aliases: Aliases::default(),
            repository: None,
            description: String::new(),
            created_by: "user-1".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
        }
    }

    #[test]
    fn a_project_brings_its_client() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let site = project(org, "Site", Some(&acme));
        let links = Links::of(None, Some(&site)).unwrap();
        assert_eq!(links.client(), Some(&acme.reference()));
        assert_eq!(links.project(), Some(&site.reference()));
    }

    #[test]
    fn the_same_client_may_be_given_with_its_project() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let site = project(org, "Site", Some(&acme));
        let links = Links::of(Some(acme.reference()), Some(&site)).unwrap();
        assert_eq!(links.client_id(), Some(acme.id));
    }

    #[test]
    fn another_client_than_the_projects_is_refused() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let globex = client(org, "Globex");
        let site = project(org, "Site", Some(&acme));
        assert!(matches!(
            Links::of(Some(globex.reference()), Some(&site)),
            Err(CoreError::Invalid(_))
        ));
        let internal = project(org, "Interne", None);
        assert!(matches!(
            Links::of(Some(acme.reference()), Some(&internal)),
            Err(CoreError::Invalid(_))
        ));
    }

    #[test]
    fn a_client_alone_and_nothing_at_all_are_fine() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let links = Links::of(Some(acme.reference()), None).unwrap();
        assert_eq!(links.client_id(), Some(acme.id));
        assert_eq!(links.project(), None);
        assert!(Links::of(None, None).unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_choice_fills_the_client_and_the_repository_from_the_project() {
        let org = OrganizationId::new();
        let acme = client(org, "Acme");
        let mut site = project(org, "Site", Some(&acme));
        site.repository = Some("acme/web".parse().unwrap());
        let site_id = site.id;
        let mut clients = MockClientRepository::new();
        let mut projects = MockProjectRepository::new();
        projects.expect_find().returning(move |_, _| {
            let site = site.clone();
            Box::pin(async move { Ok(Some(site)) })
        });

        let chosen = resolve_choice(
            &mut clients,
            &mut projects,
            org,
            LinkChoice {
                client_id: None,
                project_id: Some(site_id),
            },
        )
        .await
        .unwrap();

        assert_eq!(chosen.links.client_id(), Some(acme.id));
        assert_eq!(chosen.repository.unwrap().to_string(), "acme/web");
    }

    #[tokio::test]
    async fn an_unknown_or_archived_choice_is_invalid() {
        let org = OrganizationId::new();
        let mut archived = client(org, "Vieux");
        archived.archived_at = Some(Utc::now());
        let archived_id = archived.id;
        let mut clients = MockClientRepository::new();
        clients.expect_find().returning(move |_, id| {
            let found = (id == archived_id).then(|| archived.clone());
            Box::pin(async move { Ok(found) })
        });
        let mut projects = MockProjectRepository::new();

        for id in [archived_id, ClientId::new()] {
            let error = resolve_choice(
                &mut clients,
                &mut projects,
                org,
                LinkChoice {
                    client_id: Some(id),
                    project_id: None,
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)), "{error:?}");
        }
    }
}
