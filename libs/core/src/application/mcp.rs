use capitainerie_common::{CoreError, McpConnectorId, OrganizationId};
use capitainerie_macros::transactional;
use chrono::Utc;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        agent::SecretToken,
        authorization::Subject,
        mcp::{
            AuthorizationStart, CodeExchange, ConnectionStatus, ConnectorView, McpConnection,
            McpConnector, PendingAuthorization, RefreshExchange, ToolInfo, authorization_url,
            parse_server_url,
            ports::{
                McpAuthorizationRepository, McpConnectionRepository, McpConnectorRepository,
                McpGateway, McpOAuth,
            },
            service::{CreateConnector, McpService, NewConnector, UpdateConnector},
        },
    },
    infrastructure::mcp::postgres::{PgMcpAuthorizationRepository, PgMcpConnectionRepository},
};

const RECONNECT: &str = "la connexion a expiré, reconnectez-vous";

impl CapitainerieUseCase {
    pub fn secrets_configured(&self) -> bool {
        self.secrets.is_some()
    }

    #[transactional(mcp_connector, authz)]
    pub async fn list_mcp_connectors(
        &self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<ConnectorView>, CoreError> {
        let organization_id = subject.organization_id;
        let user = subject.id.clone();
        let connectors = McpService::new(mcp_connector_repository, authz)
            .list(subject, archived)
            .await?;
        let expiries = PgMcpConnectionRepository::new(&tx, self.secrets.as_deref())
            .expiries(organization_id, &user)
            .await?;
        Ok(connectors
            .into_iter()
            .map(|connector| {
                let connection =
                    expiries
                        .iter()
                        .find(|(id, _)| *id == connector.id)
                        .map(|(_, expires_at)| ConnectionStatus {
                            expires_at: *expires_at,
                        });
                ConnectorView {
                    connector,
                    connection,
                }
            })
            .collect())
    }

    #[transactional(mcp_connector, authz)]
    pub async fn get_mcp_connector(
        &self,
        subject: Subject,
        id: McpConnectorId,
    ) -> Result<ConnectorView, CoreError> {
        let organization_id = subject.organization_id;
        let user = subject.id.clone();
        let connector = McpService::new(mcp_connector_repository, authz)
            .get(subject, id)
            .await?;
        let connection = PgMcpConnectionRepository::new(&tx, self.secrets.as_deref())
            .expiries(organization_id, &user)
            .await?
            .into_iter()
            .find(|(connector_id, _)| *connector_id == id)
            .map(|(_, expires_at)| ConnectionStatus { expires_at });
        Ok(ConnectorView {
            connector,
            connection,
        })
    }

    pub async fn create_mcp_connector<O: McpOAuth>(
        &self,
        subject: Subject,
        command: CreateConnector,
        redirect_uri: &str,
        oauth: &O,
    ) -> Result<McpConnector, CoreError> {
        self.precheck_mcp_connector(subject.clone(), &command)
            .await?;
        let server = parse_server_url(&command.url, "l'URL du serveur")?;
        let discovery = oauth.discover(server.as_str()).await?;
        let client_id = oauth
            .register_client(
                &discovery.registration_endpoint,
                redirect_uri,
                &discovery.scopes,
            )
            .await?;
        self.insert_mcp_connector(
            subject,
            NewConnector {
                command,
                discovery,
                client_id,
            },
        )
        .await
    }

    #[transactional(mcp_connector, authz)]
    pub async fn update_mcp_connector(
        &self,
        subject: Subject,
        id: McpConnectorId,
        command: UpdateConnector,
    ) -> Result<McpConnector, CoreError> {
        McpService::new(mcp_connector_repository, authz)
            .update(subject, id, command)
            .await
    }

    #[transactional(mcp_connector, authz)]
    pub async fn start_mcp_authorization(
        &self,
        subject: Subject,
        id: McpConnectorId,
        redirect_uri: &str,
    ) -> Result<AuthorizationStart, CoreError> {
        let organization_id = subject.organization_id;
        let user = subject.id.clone();
        let connector = McpService::new(mcp_connector_repository, authz)
            .usable(subject, id)
            .await?;
        let now = Utc::now();
        let (pending, challenge) =
            PendingAuthorization::begin(&connector, &user, redirect_uri, now)?;
        let mut states = PgMcpAuthorizationRepository::new(&tx, self.secrets.as_deref());
        states.purge_expired(now).await?;
        states.insert(organization_id, &pending).await?;
        Ok(AuthorizationStart {
            authorization_url: authorization_url(
                &connector,
                redirect_uri,
                &pending.state,
                &challenge,
            )?,
            state: pending.state,
        })
    }

    pub async fn complete_mcp_authorization<O: McpOAuth>(
        &self,
        subject: Subject,
        state: &str,
        code: &str,
        oauth: &O,
    ) -> Result<McpConnectorId, CoreError> {
        let organization_id = subject.organization_id;
        let user = subject.id.clone();
        let pending = self
            .take_mcp_authorization(organization_id, state)
            .await?
            .ok_or_else(|| {
                CoreError::Invalid("autorisation inconnue ou déjà utilisée, recommencez".into())
            })?;
        pending.check(&user, Utc::now())?;
        let connector = self
            .usable_mcp_connector(subject, pending.connector_id)
            .await?;
        let tokens = oauth
            .exchange_code(&CodeExchange {
                token_endpoint: &connector.token_endpoint,
                client_id: &connector.client_id,
                resource: &connector.resource,
                code,
                code_verifier: &pending.code_verifier,
                redirect_uri: &pending.redirect_uri,
            })
            .await?;
        let now = Utc::now();
        self.store_mcp_connection(
            organization_id,
            McpConnection {
                connector_id: connector.id,
                subject: user,
                tokens,
                created_at: now,
                updated_at: now,
            },
        )
        .await?;
        Ok(connector.id)
    }

    #[transactional(mcp_connector, authz)]
    pub async fn disconnect_mcp_connector(
        &self,
        subject: Subject,
        id: McpConnectorId,
    ) -> Result<(), CoreError> {
        let organization_id = subject.organization_id;
        let user = subject.id.clone();
        McpService::new(mcp_connector_repository, authz)
            .get(subject, id)
            .await?;
        PgMcpConnectionRepository::new(&tx, self.secrets.as_deref())
            .delete(organization_id, id, &user)
            .await?;
        Ok(())
    }

    pub async fn test_mcp_connector<O: McpOAuth, G: McpGateway>(
        &self,
        subject: Subject,
        id: McpConnectorId,
        oauth: &O,
        gateway: &G,
    ) -> Result<Vec<ToolInfo>, CoreError> {
        let organization_id = subject.organization_id;
        let user = subject.id.clone();
        let connector = self.usable_mcp_connector(subject, id).await?;
        let token = self
            .mcp_access_token(organization_id, id, &user, oauth)
            .await?;
        gateway.list_tools(&connector.url, &token).await
    }

    pub async fn mcp_access_token<O: McpOAuth>(
        &self,
        organization_id: OrganizationId,
        connector_id: McpConnectorId,
        subject: &str,
        oauth: &O,
    ) -> Result<SecretToken, CoreError> {
        let (connector, connection) = self
            .stored_mcp_connection(organization_id, connector_id, subject)
            .await?;
        if !connection.tokens.needs_refresh(Utc::now()) {
            return Ok(connection.tokens.access_token);
        }
        let Some(refresh_token) = &connection.tokens.refresh_token else {
            return Err(CoreError::Conflict(RECONNECT.into()));
        };
        let refreshed = oauth
            .refresh(&RefreshExchange {
                token_endpoint: &connector.token_endpoint,
                client_id: &connector.client_id,
                resource: &connector.resource,
                refresh_token,
            })
            .await;
        let fresh = match refreshed {
            Ok(fresh) => fresh,
            Err(error) => {
                let (_, current) = self
                    .stored_mcp_connection(organization_id, connector_id, subject)
                    .await?;
                if current.tokens.access_token != connection.tokens.access_token
                    && !current.tokens.needs_refresh(Utc::now())
                {
                    return Ok(current.tokens.access_token);
                }
                return Err(error);
            }
        };
        let renewed = McpConnection {
            tokens: connection.tokens.renewed_by(fresh),
            updated_at: Utc::now(),
            ..connection
        };
        let token = renewed.tokens.access_token.clone();
        self.store_mcp_connection(organization_id, renewed).await?;
        Ok(token)
    }

    #[transactional(mcp_connector, authz)]
    async fn precheck_mcp_connector(
        &self,
        subject: Subject,
        command: &CreateConnector,
    ) -> Result<(), CoreError> {
        McpService::new(mcp_connector_repository, authz)
            .precheck(subject, command)
            .await
    }

    #[transactional(mcp_connector, authz)]
    async fn insert_mcp_connector(
        &self,
        subject: Subject,
        new: NewConnector,
    ) -> Result<McpConnector, CoreError> {
        McpService::new(mcp_connector_repository, authz)
            .create(subject, new)
            .await
    }

    #[transactional(mcp_connector, authz)]
    async fn usable_mcp_connector(
        &self,
        subject: Subject,
        id: McpConnectorId,
    ) -> Result<McpConnector, CoreError> {
        McpService::new(mcp_connector_repository, authz)
            .usable(subject, id)
            .await
    }

    #[transactional()]
    async fn take_mcp_authorization(
        &self,
        organization_id: OrganizationId,
        state: &str,
    ) -> Result<Option<PendingAuthorization>, CoreError> {
        PgMcpAuthorizationRepository::new(&tx, self.secrets.as_deref())
            .take(organization_id, state)
            .await
    }

    #[transactional()]
    async fn store_mcp_connection(
        &self,
        organization_id: OrganizationId,
        connection: McpConnection,
    ) -> Result<(), CoreError> {
        PgMcpConnectionRepository::new(&tx, self.secrets.as_deref())
            .upsert(organization_id, &connection)
            .await
    }

    #[transactional(mcp_connector)]
    async fn stored_mcp_connection(
        &self,
        organization_id: OrganizationId,
        connector_id: McpConnectorId,
        subject: &str,
    ) -> Result<(McpConnector, McpConnection), CoreError> {
        let mut mcp_connector_repository = mcp_connector_repository;
        let connector = mcp_connector_repository
            .find(organization_id, connector_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        let connection = PgMcpConnectionRepository::new(&tx, self.secrets.as_deref())
            .find(organization_id, connector_id, subject)
            .await?
            .ok_or_else(|| CoreError::Conflict("ce connecteur n'est pas connecté".into()))?;
        Ok((connector, connection))
    }
}
