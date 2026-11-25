use std::sync::Arc;
use std::time::Duration;

use capitainerie_common::CoreError;
use capitainerie_core::domain::agent::SecretToken;
use capitainerie_core::domain::agent_identity::ports::AgentIdentityProvisioner;
use capitainerie_core::domain::agent_identity::{AgentClientId, ProvisionedClient};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::Deserialize;
use serde_json::json;

use super::exchange::{ClientCredentials, ExchangeArgs};
use crate::AuthError;

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct Discovery {
    token_endpoint: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct Client {
    id: String,
    enabled: bool,
    public_client: bool,
    service_account_enabled: bool,
    token_exchange_enabled: bool,
}

#[derive(Deserialize)]
struct Page {
    data: Vec<Client>,
}

#[derive(Deserialize)]
struct Secret {
    client_secret: Option<String>,
}

struct Inner {
    credentials: ClientCredentials,
    api_client_id: String,
    mcp_audience: String,
    http: reqwest::Client,
}

/// Creates agents' clients through FerrisKey's admin API, signed in as the
/// realm's provisioner client: a service account whose only permission is to
/// manage clients.
#[derive(Clone)]
pub struct FerrisKeyProvisioner {
    inner: Arc<Inner>,
}

fn internal(message: impl std::fmt::Display) -> CoreError {
    CoreError::Internal(format!("agent identity: {message}"))
}

/// The admin API of a realm lives under its issuer: `{url}/realms/{realm}`.
struct Admin<'a> {
    http: &'a reqwest::Client,
    base: &'a str,
    token: SecretToken,
}

impl Admin<'_> {
    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(self.token.expose())
    }

    async fn send(&self, request: RequestBuilder, what: &str) -> Result<Response, CoreError> {
        let response = request
            .send()
            .await
            .map_err(|e| internal(format!("{what}: {}", e.without_url())))?;
        match response.status() {
            status if status.is_success() => Ok(response),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(CoreError::Forbidden {
                reason: Some(format!(
                    "agent identity: the provisioner client may not {what}"
                )),
            }),
            status => Err(internal(format!("{what}: status {status}"))),
        }
    }

    async fn find(&self, client_id: &str) -> Result<Option<Client>, CoreError> {
        let request = self
            .request(Method::GET, "/clients")
            .query(&[("client_id_exact", client_id)]);
        let page: Page = self
            .send(request, "list clients")
            .await?
            .json()
            .await
            .map_err(|e| internal(format!("list clients: {}", e.without_url())))?;
        Ok(page.data.into_iter().next())
    }

    async fn create(&self, client_id: &str, name: &str) -> Result<Client, CoreError> {
        let request = self.request(Method::POST, "/clients").json(&json!({
            "client_id": client_id,
            "name": name,
            "client_type": "confidential",
            "public_client": false,
            "service_account_enabled": true,
            "direct_access_grants_enabled": false,
            "token_exchange_enabled": true,
            "enabled": true,
        }));
        self.send(request, "create a client")
            .await?
            .json()
            .await
            .map_err(|e| internal(format!("create a client: {}", e.without_url())))
    }

    async fn patch(&self, client_uuid: &str, body: serde_json::Value) -> Result<(), CoreError> {
        let request = self
            .request(Method::PATCH, &format!("/clients/{client_uuid}"))
            .json(&body);
        self.send(request, "update a client").await.map(drop)
    }

    /// A policy already there answers 409: that is the state we want.
    async fn ensure_policy(
        &self,
        client_uuid: &str,
        target_audience: &str,
        impersonation: bool,
        delegation: bool,
    ) -> Result<(), CoreError> {
        let response = self
            .request(
                Method::POST,
                &format!("/clients/{client_uuid}/token-exchange-policies"),
            )
            .json(&json!({
                "target_audience": target_audience,
                "allow_impersonation": impersonation,
                "allow_delegation": delegation,
            }))
            .send()
            .await
            .map_err(|e| internal(format!("create an exchange policy: {}", e.without_url())))?;
        match response.status() {
            StatusCode::CONFLICT => Ok(()),
            status if status.is_success() => Ok(()),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(CoreError::Forbidden {
                reason: Some(
                    "agent identity: the provisioner client may not create an exchange policy"
                        .into(),
                ),
            }),
            status => Err(internal(format!(
                "create an exchange policy: status {status}"
            ))),
        }
    }

    async fn secret(&self, client_uuid: &str) -> Result<SecretToken, CoreError> {
        let request = self.request(
            Method::GET,
            &format!("/clients/{client_uuid}/client-secret"),
        );
        let secret: Secret = self
            .send(request, "read a client secret")
            .await?
            .json()
            .await
            .map_err(|e| internal(format!("read a client secret: {}", e.without_url())))?;
        secret
            .client_secret
            .map(SecretToken::new)
            .ok_or_else(|| internal("the client has no secret"))
    }
}

impl FerrisKeyProvisioner {
    pub fn new(
        credentials: ClientCredentials,
        api_client_id: impl Into<String>,
        mcp_audience: impl Into<String>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            inner: Arc::new(Inner {
                credentials,
                api_client_id: api_client_id.into(),
                mcp_audience: mcp_audience.into(),
                http,
            }),
        }
    }

    async fn admin<'a>(&'a self, issuer: &'a str) -> Result<Admin<'a>, CoreError> {
        let base = issuer.trim_end_matches('/');
        let http = &self.inner.http;
        let discovery: Discovery = http
            .get(format!("{base}/.well-known/openid-configuration"))
            .send()
            .await
            .and_then(Response::error_for_status)
            .map_err(|e| internal(format!("discovery: {}", e.without_url())))?
            .json()
            .await
            .map_err(|e| internal(format!("discovery: {}", e.without_url())))?;
        let credentials = &self.inner.credentials;
        let response = http
            .post(&discovery.token_endpoint)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", credentials.client_id.as_str()),
                ("client_secret", credentials.client_secret.expose()),
            ])
            .send()
            .await
            .map_err(|e| internal(format!("sign in: {}", e.without_url())))?;
        if !response.status().is_success() {
            return Err(CoreError::Forbidden {
                reason: Some(format!(
                    "agent identity: the provisioner client {} cannot sign in (status {})",
                    credentials.client_id,
                    response.status()
                )),
            });
        }
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| internal(format!("sign in: {}", e.without_url())))?;
        Ok(Admin {
            http,
            base,
            token: SecretToken::new(token.access_token),
        })
    }
}

impl AgentIdentityProvisioner for FerrisKeyProvisioner {
    #[tracing::instrument(skip_all, fields(client_id = client_id.as_str()), err)]
    async fn provision(
        &self,
        issuer: &str,
        client_id: &AgentClientId,
        name: &str,
    ) -> Result<ProvisionedClient, CoreError> {
        let admin = self.admin(issuer).await?;
        let client = match admin.find(client_id.as_str()).await? {
            Some(client) => {
                if client.public_client || !client.service_account_enabled {
                    return Err(CoreError::Conflict(format!(
                        "a client {} exists and is not an agent's client",
                        client_id.as_str()
                    )));
                }
                if !client.enabled || !client.token_exchange_enabled {
                    admin
                        .patch(
                            &client.id,
                            json!({"enabled": true, "token_exchange_enabled": true}),
                        )
                        .await?;
                }
                client
            }
            None => admin.create(client_id.as_str(), name).await?,
        };
        admin
            .ensure_policy(&client.id, &self.inner.mcp_audience, false, true)
            .await?;
        let api = admin
            .find(&self.inner.api_client_id)
            .await?
            .ok_or_else(|| internal(format!("no client {}", self.inner.api_client_id)))?;
        admin
            .ensure_policy(&api.id, client_id.as_str(), true, false)
            .await?;
        let secret = admin.secret(&client.id).await?;
        Ok(ProvisionedClient {
            client_uuid: client.id,
            secret,
        })
    }

    #[tracing::instrument(skip_all, fields(client_uuid = client_uuid, enabled = enabled), err)]
    async fn set_enabled(
        &self,
        issuer: &str,
        client_uuid: &str,
        enabled: bool,
    ) -> Result<(), CoreError> {
        let admin = self.admin(issuer).await?;
        admin.patch(client_uuid, json!({"enabled": enabled})).await
    }
}

/// The provisioner's own client, one pair per deployment. Unset, agents keep
/// a pending identity.
#[derive(Clone, clap::Args)]
pub struct ProvisionerArgs {
    #[arg(long = "provisioner-client-id", env = "PROVISIONER_CLIENT_ID")]
    pub provisioner_client_id: Option<String>,
    #[arg(long = "provisioner-client-secret", env = "PROVISIONER_CLIENT_SECRET")]
    pub provisioner_client_secret: Option<String>,
}

impl std::fmt::Debug for ProvisionerArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProvisionerArgs")
            .field("client_id", &self.provisioner_client_id)
            .field(
                "client_secret",
                &self.provisioner_client_secret.as_ref().map(|_| "***"),
            )
            .finish()
    }
}

impl ProvisionerArgs {
    /// `None` when no provisioner is configured. The API's client, which
    /// the exchange args name, is the one allowed to address the agents.
    pub fn provisioner(
        &self,
        exchange: &ExchangeArgs,
    ) -> Result<Option<FerrisKeyProvisioner>, AuthError> {
        let credentials = match (&self.provisioner_client_id, &self.provisioner_client_secret) {
            (Some(id), Some(secret)) => ClientCredentials {
                client_id: id.clone(),
                client_secret: SecretToken::new(secret.as_str()),
            },
            (None, None) => return Ok(None),
            _ => {
                return Err(AuthError::Config(
                    "provisioner client id and secret must be set together".to_owned(),
                ));
            }
        };
        let api_client_id = exchange.client_id.clone().ok_or_else(|| {
            AuthError::Config(
                "EXCHANGE_CLIENT_ID is required with a provisioner: it names the client allowed to act as the agents".to_owned(),
            )
        })?;
        Ok(Some(FerrisKeyProvisioner::new(
            credentials,
            api_client_id,
            exchange.mcp_audience.clone(),
        )))
    }
}
