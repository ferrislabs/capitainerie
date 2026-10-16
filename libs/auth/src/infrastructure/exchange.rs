use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use capitainerie_common::CoreError;
use capitainerie_core::domain::agent::SecretToken;
use capitainerie_core::domain::delegation::DelegatedToken;
use capitainerie_core::domain::delegation::ports::TokenExchanger;
use chrono::Utc;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::domain::errors::AuthError;

const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const GRANT_CLIENT_CREDENTIALS: &str = "client_credentials";
const GRANT_TOKEN_EXCHANGE: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const TOKEN_TYPE_ACCESS: &str = "urn:ietf:params:oauth:token-type:access_token";

#[derive(Clone)]
pub struct ClientCredentials {
    pub client_id: String,
    pub client_secret: SecretToken,
}

impl std::fmt::Debug for ClientCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientCredentials")
            .field("client_id", &self.client_id)
            .field("client_secret", &"***")
            .finish()
    }
}

#[derive(Deserialize)]
struct Discovery {
    token_endpoint: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
    error_description: Option<String>,
}

struct Inner {
    requester: Option<ClientCredentials>,
    agents: HashMap<String, ClientCredentials>,
    mcp_audience: String,
    http: reqwest::Client,
    endpoints: RwLock<HashMap<String, String>>,
}

#[derive(Clone)]
pub struct FerrisKeyTokenExchange {
    inner: Arc<Inner>,
}

fn internal(message: impl std::fmt::Display) -> CoreError {
    CoreError::Internal(format!("token exchange: {message}"))
}

impl FerrisKeyTokenExchange {
    pub fn new(
        requester: Option<ClientCredentials>,
        agents: Vec<ClientCredentials>,
        mcp_audience: impl Into<String>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            inner: Arc::new(Inner {
                requester,
                agents: agents
                    .into_iter()
                    .map(|c| (c.client_id.clone(), c))
                    .collect(),
                mcp_audience: mcp_audience.into(),
                http,
                endpoints: RwLock::new(HashMap::new()),
            }),
        }
    }

    async fn token_endpoint(&self, issuer: &str) -> Result<String, CoreError> {
        if let Some(endpoint) = self.inner.endpoints.read().await.get(issuer) {
            return Ok(endpoint.clone());
        }
        let url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        let discovery: Discovery = self
            .inner
            .http
            .get(&url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| internal(format!("discovery: {}", e.without_url())))?
            .json()
            .await
            .map_err(|e| internal(format!("discovery: {}", e.without_url())))?;
        self.inner
            .endpoints
            .write()
            .await
            .insert(issuer.to_owned(), discovery.token_endpoint.clone());
        Ok(discovery.token_endpoint)
    }

    async fn post(&self, issuer: &str, form: &[(&str, &str)]) -> Result<DelegatedToken, CoreError> {
        let endpoint = self.token_endpoint(issuer).await?;
        let response = self
            .inner
            .http
            .post(&endpoint)
            .form(form)
            .send()
            .await
            .map_err(|e| internal(e.without_url()))?;
        let status = response.status();
        if status.is_client_error() {
            let body: OAuthError = response
                .json()
                .await
                .map_err(|e| internal(format!("status {status}: {}", e.without_url())))?;
            let reason = match body.error_description {
                Some(description) => format!("{}: {description}", body.error),
                None => body.error,
            };
            return Err(CoreError::Forbidden {
                reason: Some(reason),
            });
        }
        if !status.is_success() {
            return Err(internal(format!("status {status}")));
        }
        let body: TokenResponse = response
            .json()
            .await
            .map_err(|e| internal(e.without_url()))?;
        let expires_in = body
            .expires_in
            .ok_or_else(|| internal("missing expires_in"))?;
        Ok(DelegatedToken {
            token: SecretToken::new(body.access_token),
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in),
        })
    }
}

impl TokenExchanger for FerrisKeyTokenExchange {
    #[tracing::instrument(skip_all, fields(audience = agent_client_id, agent = agent_client_id), err)]
    async fn for_agent(
        &self,
        issuer: &str,
        user_token: &SecretToken,
        agent_client_id: &str,
    ) -> Result<DelegatedToken, CoreError> {
        let requester = self
            .inner
            .requester
            .as_ref()
            .ok_or_else(|| internal("no requester client configured"))?;
        self.post(
            issuer,
            &[
                ("grant_type", GRANT_TOKEN_EXCHANGE),
                ("client_id", &requester.client_id),
                ("client_secret", requester.client_secret.expose()),
                ("subject_token", user_token.expose()),
                ("subject_token_type", TOKEN_TYPE_ACCESS),
                ("audience", agent_client_id),
            ],
        )
        .await
    }

    #[tracing::instrument(skip_all, fields(audience = %self.inner.mcp_audience, agent = agent_client_id), err)]
    async fn for_mcp(
        &self,
        issuer: &str,
        agent_token: &SecretToken,
        agent_client_id: &str,
    ) -> Result<DelegatedToken, CoreError> {
        let agent = self
            .inner
            .agents
            .get(agent_client_id)
            .ok_or_else(|| internal(format!("unknown agent {agent_client_id}")))?;
        let actor = self
            .post(
                issuer,
                &[
                    ("grant_type", GRANT_CLIENT_CREDENTIALS),
                    ("client_id", &agent.client_id),
                    ("client_secret", agent.client_secret.expose()),
                ],
            )
            .await?;
        self.post(
            issuer,
            &[
                ("grant_type", GRANT_TOKEN_EXCHANGE),
                ("client_id", &agent.client_id),
                ("client_secret", agent.client_secret.expose()),
                ("subject_token", agent_token.expose()),
                ("subject_token_type", TOKEN_TYPE_ACCESS),
                ("actor_token", actor.token.expose()),
                ("actor_token_type", TOKEN_TYPE_ACCESS),
                ("audience", &self.inner.mcp_audience),
            ],
        )
        .await
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct ExchangeArgs {
    #[arg(long = "exchange-client-id", env = "EXCHANGE_CLIENT_ID")]
    pub client_id: Option<String>,
    #[arg(long = "exchange-client-secret", env = "EXCHANGE_CLIENT_SECRET")]
    pub client_secret: Option<String>,
    #[arg(
        long = "agent-credentials",
        env = "AGENT_CREDENTIALS",
        value_delimiter = ','
    )]
    pub agent_credentials: Vec<String>,
    #[arg(
        long = "mcp-audience",
        env = "MCP_AUDIENCE",
        default_value = "capitainerie-mcp"
    )]
    pub mcp_audience: String,
}

impl ExchangeArgs {
    pub fn exchanger(&self) -> Result<FerrisKeyTokenExchange, AuthError> {
        let requester = match (&self.client_id, &self.client_secret) {
            (Some(id), Some(secret)) => Some(ClientCredentials {
                client_id: id.clone(),
                client_secret: SecretToken::new(secret.as_str()),
            }),
            (None, None) => None,
            _ => {
                return Err(AuthError::Config(
                    "exchange client id and secret must be set together".to_owned(),
                ));
            }
        };
        let agents = self
            .agent_credentials
            .iter()
            .map(|entry| match entry.split_once('=') {
                Some((id, secret)) if !id.is_empty() && !secret.is_empty() => {
                    Ok(ClientCredentials {
                        client_id: id.to_owned(),
                        client_secret: SecretToken::new(secret),
                    })
                }
                _ => Err(AuthError::Config(
                    "agent credentials must be `client_id=secret`".to_owned(),
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FerrisKeyTokenExchange::new(
            requester,
            agents,
            self.mcp_audience.clone(),
        ))
    }
}
