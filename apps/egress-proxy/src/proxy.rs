use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Router,
    http::StatusCode,
    routing::{any, get},
};
use capitainerie_core::domain::execution::TaskGrant;
use chrono::Utc;

use crate::{anthropic, auth, credentials::Credentials, error::ProxyError, git};

const GRANT_CACHE_TTL: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub anthropic_base: String,
    pub github_base: String,
}

pub struct Proxy<C> {
    pub(crate) credentials: C,
    pub(crate) http: reqwest::Client,
    pub(crate) config: ProxyConfig,
    cache: Mutex<HashMap<[u8; 32], (TaskGrant, Instant)>>,
}

impl<C: Credentials> Proxy<C> {
    pub fn new(credentials: C, config: ProxyConfig) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self::with_http(credentials, config, http))
    }

    pub fn with_http(credentials: C, config: ProxyConfig, http: reqwest::Client) -> Self {
        Self {
            credentials,
            http,
            config: ProxyConfig {
                anthropic_base: config.anthropic_base.trim_end_matches('/').to_owned(),
                github_base: config.github_base.trim_end_matches('/').to_owned(),
            },
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn authenticate(&self, token: &str) -> Result<TaskGrant, ProxyError> {
        let hash = auth::sha256(token);
        let now = Utc::now();
        if let Some(grant) = self.cached(&hash)
            && grant.is_usable(now)
        {
            return Ok(grant);
        }
        match self.credentials.grant(hash).await? {
            Some(grant) if grant.is_usable(now) => {
                self.remember(hash, &grant);
                Ok(grant)
            }
            Some(_) => {
                self.forget(&hash);
                Err(ProxyError::Unauthorized {
                    reason: "grant revoked or expired",
                })
            }
            None => Err(ProxyError::Unauthorized {
                reason: "unknown token",
            }),
        }
    }

    fn cached(&self, hash: &[u8; 32]) -> Option<TaskGrant> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .get(hash)
            .filter(|(_, at)| at.elapsed() < GRANT_CACHE_TTL)
            .map(|(grant, _)| grant.clone())
    }

    fn remember(&self, hash: [u8; 32], grant: &TaskGrant) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.retain(|_, (_, at)| at.elapsed() < GRANT_CACHE_TTL);
        cache.insert(hash, (grant.clone(), Instant::now()));
    }

    fn forget(&self, hash: &[u8; 32]) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.remove(hash);
    }
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub fn router<C: Credentials>(proxy: Proxy<C>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/anthropic/{*path}", any(anthropic::handle::<C>))
        .route("/git/{owner}/{repo}/{*rest}", any(git::handle::<C>))
        .with_state(Arc::new(proxy))
}
