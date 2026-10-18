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
use capitainerie_common::RunId;
use capitainerie_core::domain::execution::TaskGrant;
use chrono::Utc;

use crate::{anthropic, auth, credentials::Credentials, error::ProxyError, git};

const GRANT_CACHE_TTL: Duration = Duration::from_secs(10);
const UNKNOWN_TOKEN_TTL: Duration = Duration::from_secs(10);
const UNKNOWN_TOKENS_MAX: usize = 10_000;
pub const MAX_IN_FLIGHT: usize = 16;

type InFlight = Arc<Mutex<HashMap<RunId, usize>>>;

/// One request being served for a grant; released when the response body ends.
pub struct Permit {
    in_flight: InFlight,
    run_id: RunId,
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut in_flight = self.in_flight.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = in_flight.get_mut(&self.run_id) {
            *count -= 1;
            if *count == 0 {
                in_flight.remove(&self.run_id);
            }
        }
    }
}

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
    unknown: Mutex<HashMap<[u8; 32], Instant>>,
    in_flight: InFlight,
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
            unknown: Mutex::new(HashMap::new()),
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn acquire(&self, run_id: RunId) -> Result<Permit, ProxyError> {
        let mut in_flight = self.in_flight.lock().unwrap_or_else(|e| e.into_inner());
        let count = in_flight.entry(run_id).or_insert(0);
        if *count >= MAX_IN_FLIGHT {
            return Err(ProxyError::TooManyRequests { run_id });
        }
        *count += 1;
        Ok(Permit {
            in_flight: Arc::clone(&self.in_flight),
            run_id,
        })
    }

    pub(crate) async fn authenticate(&self, token: &str) -> Result<TaskGrant, ProxyError> {
        let hash = auth::sha256(token);
        let now = Utc::now();
        if let Some(grant) = self.cached(&hash)
            && grant.is_usable(now)
        {
            return Ok(grant);
        }
        if self.known_unusable(&hash) {
            return Err(ProxyError::Unauthorized {
                reason: "unknown token",
            });
        }
        match self.credentials.grant(hash).await? {
            Some(grant) if grant.is_usable(now) => {
                self.remember(hash, &grant);
                Ok(grant)
            }
            Some(_) => {
                self.forget(&hash);
                self.reject(hash);
                Err(ProxyError::Unauthorized {
                    reason: "grant revoked or expired",
                })
            }
            None => {
                self.reject(hash);
                Err(ProxyError::Unauthorized {
                    reason: "unknown token",
                })
            }
        }
    }

    fn known_unusable(&self, hash: &[u8; 32]) -> bool {
        let unknown = self.unknown.lock().unwrap_or_else(|e| e.into_inner());
        unknown
            .get(hash)
            .is_some_and(|at| at.elapsed() < UNKNOWN_TOKEN_TTL)
    }

    fn reject(&self, hash: [u8; 32]) {
        let mut unknown = self.unknown.lock().unwrap_or_else(|e| e.into_inner());
        unknown.retain(|_, at| at.elapsed() < UNKNOWN_TOKEN_TTL);
        if unknown.len() >= UNKNOWN_TOKENS_MAX {
            unknown.clear();
        }
        unknown.insert(hash, Instant::now());
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
