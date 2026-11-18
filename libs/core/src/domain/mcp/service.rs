use capitainerie_common::{CoreError, McpConnectorId};
use chrono::Utc;

use super::{
    Discovery, McpConnector, SCOPES_MAX_COUNT, parse_server_url, ports::McpConnectorRepository,
    slugify,
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    crew::Slug,
    label::Name,
};

#[derive(Debug, Clone)]
pub struct CreateConnector {
    pub url: String,
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewConnector {
    pub command: CreateConnector,
    pub discovery: Discovery,
    pub client_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateConnector {
    pub name: Option<String>,
    pub archived: Option<bool>,
}

pub struct McpService<'a, R, A> {
    connectors: R,
    authorizer: &'a A,
}

impl<'a, R, A> McpService<'a, R, A>
where
    R: McpConnectorRepository,
    A: Authorizer,
{
    pub fn new(connectors: R, authorizer: &'a A) -> Self {
        Self {
            connectors,
            authorizer,
        }
    }

    pub async fn precheck(
        mut self,
        subject: Subject,
        command: &CreateConnector,
    ) -> Result<(), CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::McpConnectorManage, None)
            .await?;
        let (_, slug) = identity(command)?;
        let taken = self.connectors.list(organization_id, None).await?;
        if taken.iter().any(|connector| connector.slug == slug) {
            return Err(slug_taken(&slug));
        }
        Ok(())
    }

    pub async fn list(
        mut self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<McpConnector>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::McpConnect, None).await?;
        self.connectors
            .list(organization_id, Some(archived.unwrap_or(false)))
            .await
    }

    pub async fn get(
        mut self,
        subject: Subject,
        id: McpConnectorId,
    ) -> Result<McpConnector, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::McpConnect, Some(id)).await?;
        self.connectors
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    pub async fn usable(
        self,
        subject: Subject,
        id: McpConnectorId,
    ) -> Result<McpConnector, CoreError> {
        let connector = self.get(subject, id).await?;
        if connector.archived {
            return Err(CoreError::Conflict("ce connecteur est archivé".into()));
        }
        Ok(connector)
    }

    pub async fn create(
        mut self,
        subject: Subject,
        new: NewConnector,
    ) -> Result<McpConnector, CoreError> {
        self.require(subject.clone(), Action::McpConnectorManage, None)
            .await?;
        let url = parse_server_url(&new.command.url, "l'URL du serveur")?;
        let discovery = new.discovery;
        parse_server_url(
            &discovery.authorization_endpoint,
            "l'endpoint d'autorisation",
        )?;
        parse_server_url(&discovery.token_endpoint, "l'endpoint de jeton")?;
        if discovery.scopes.len() > SCOPES_MAX_COUNT {
            return Err(CoreError::Invalid(
                "le serveur annonce trop de portées".into(),
            ));
        }
        let (name, slug) = identity(&new.command)?;
        let now = Utc::now();
        let connector = McpConnector {
            id: McpConnectorId::new(),
            organization_id: subject.organization_id,
            slug,
            name: name.as_str().to_owned(),
            url: url.into(),
            resource: discovery.resource,
            issuer: discovery.issuer,
            authorization_endpoint: discovery.authorization_endpoint,
            token_endpoint: discovery.token_endpoint,
            client_id: new.client_id,
            scopes: discovery.scopes,
            archived: false,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
        };
        self.connectors.insert(&connector).await?;
        Ok(connector)
    }

    pub async fn update(
        mut self,
        subject: Subject,
        id: McpConnectorId,
        command: UpdateConnector,
    ) -> Result<McpConnector, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::McpConnectorManage, Some(id))
            .await?;
        let mut connector = self
            .connectors
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if command.name.is_none() && command.archived.is_none() {
            return Err(CoreError::Invalid(
                "nothing to update: give at least one field".into(),
            ));
        }
        if let Some(name) = command.name {
            connector.name = Name::try_from(name)?.as_str().to_owned();
        }
        if let Some(archived) = command.archived {
            connector.archived = archived;
        }
        connector.updated_at = Utc::now();
        self.connectors.update(&connector).await?;
        Ok(connector)
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        id: Option<McpConnectorId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(
                subject,
                action,
                ResourceKind::McpConnector,
                id.map(|id| id.0),
            ),
        )
        .await
    }
}

fn identity(command: &CreateConnector) -> Result<(Name, Slug), CoreError> {
    let url = parse_server_url(&command.url, "l'URL du serveur")?;
    let name = Name::try_from(
        command
            .name
            .clone()
            .unwrap_or_else(|| url.host_str().unwrap_or_default().to_owned()),
    )?;
    let slug = match &command.slug {
        Some(slug) => Slug::try_from(slug.clone())?,
        None => Slug::try_from(slugify(name.as_str()))?,
    };
    Ok((name, slug))
}

fn slug_taken(slug: &Slug) -> CoreError {
    CoreError::Conflict(format!(
        "un connecteur de slug {} existe déjà",
        slug.as_str()
    ))
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        mcp::{ports::MockMcpConnectorRepository, tests::connector},
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

    fn discovery() -> Discovery {
        Discovery {
            resource: "https://demo.mestier.fr/mcp".into(),
            issuer: "https://auth.example/realms/mestier".into(),
            authorization_endpoint: "https://auth.example/realms/mestier/auth".into(),
            token_endpoint: "https://auth.example/realms/mestier/token".into(),
            registration_endpoint: "https://auth.example/realms/mestier/register".into(),
            scopes: vec!["mestier.read".into()],
        }
    }

    fn new_connector(name: Option<&str>, slug: Option<&str>) -> NewConnector {
        NewConnector {
            command: CreateConnector {
                url: "https://demo.mestier.fr/mcp".into(),
                name: name.map(str::to_owned),
                slug: slug.map(str::to_owned),
            },
            discovery: discovery(),
            client_id: "client-1".into(),
        }
    }

    #[tokio::test]
    async fn a_connector_takes_its_name_and_slug_from_the_url_when_none_is_given() {
        let mut repository = MockMcpConnectorRepository::new();
        repository
            .expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = allow();

        let created = McpService::new(repository, &authorizer)
            .create(human(OrganizationId::new()), new_connector(None, None))
            .await
            .unwrap();

        assert_eq!(created.name, "demo.mestier.fr");
        assert_eq!(created.slug.as_str(), "demo-mestier-fr");
        assert_eq!(created.client_id, "client-1");
        assert_eq!(created.resource, "https://demo.mestier.fr/mcp");
        assert!(!created.archived);
    }

    #[tokio::test]
    async fn invalid_input_stores_nothing() {
        let authorizer = allow();
        let mut bad_url = new_connector(None, None);
        bad_url.command.url = "ftp://x".into();
        let bad_slug = new_connector(Some("Ok"), Some("Not Kebab"));
        let bad_name = new_connector(Some(" "), None);
        let mut bad_endpoint = new_connector(None, None);
        bad_endpoint.discovery.token_endpoint = "javascript:1".into();
        for invalid in [bad_url, bad_slug, bad_name, bad_endpoint] {
            let error = McpService::new(MockMcpConnectorRepository::new(), &authorizer)
                .create(human(OrganizationId::new()), invalid)
                .await
                .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)), "{error}");
        }
    }

    #[tokio::test]
    async fn a_denied_caller_changes_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "humans only".into(),
                })
            })
        });
        let error = McpService::new(MockMcpConnectorRepository::new(), &authorizer)
            .create(human(OrganizationId::new()), new_connector(None, None))
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }

    #[tokio::test]
    async fn an_archived_connector_cannot_be_used_and_an_empty_update_is_refused() {
        let organization_id = OrganizationId::new();
        let mut archived = connector(organization_id, "old");
        archived.archived = true;
        let id = archived.id;
        let mut repository = MockMcpConnectorRepository::new();
        let stored = archived.clone();
        repository.expect_find().returning(move |_, _| {
            let found = stored.clone();
            Box::pin(async move { Ok(Some(found)) })
        });
        let authorizer = allow();

        let error = McpService::new(repository, &authorizer)
            .usable(human(organization_id), id)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Conflict(_)));

        let mut repository = MockMcpConnectorRepository::new();
        repository.expect_find().returning(move |_, _| {
            let found = archived.clone();
            Box::pin(async move { Ok(Some(found)) })
        });
        let error = McpService::new(repository, &authorizer)
            .update(human(organization_id), id, UpdateConnector::default())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn a_taken_slug_is_refused_before_any_network_call() {
        let organization_id = OrganizationId::new();
        let existing = connector(organization_id, "demo-mestier-fr");
        let mut repository = MockMcpConnectorRepository::new();
        repository.expect_list().returning(move |_, _| {
            let all = vec![existing.clone()];
            Box::pin(async move { Ok(all) })
        });
        let authorizer = allow();

        let error = McpService::new(repository, &authorizer)
            .precheck(human(organization_id), &new_connector(None, None).command)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Conflict(_)));

        let mut free = MockMcpConnectorRepository::new();
        free.expect_list()
            .returning(|_, _| Box::pin(async { Ok(vec![]) }));
        McpService::new(free, &authorizer)
            .precheck(human(organization_id), &new_connector(None, None).command)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn an_unknown_connector_is_not_found() {
        let mut repository = MockMcpConnectorRepository::new();
        repository
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        let authorizer = allow();
        let error = McpService::new(repository, &authorizer)
            .get(human(OrganizationId::new()), McpConnectorId::new())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::NotFound));
    }
}
