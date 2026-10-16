use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::jwk::{AlgorithmParameters, Jwk, PublicKeyUse};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::domain::claims::Claims;
use crate::domain::errors::AuthError;
use crate::domain::identity::{AcceptedClients, Identity};
use crate::domain::ports::TokenVerifier;

const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const EXPIRY_LEEWAY_SECS: u64 = 30;
const BEARER_TYP: &str = "Bearer";

#[derive(Deserialize)]
struct IssuerOnly {
    iss: String,
}

pub fn unverified_issuer(token: &str) -> Result<String, AuthError> {
    let data = jsonwebtoken::dangerous::insecure_decode::<IssuerOnly>(token)
        .map_err(|e| AuthError::Malformed(e.to_string()))?;
    Ok(data.claims.iss)
}

#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    jwks_uri: String,
}

#[derive(Deserialize)]
struct RawJwks {
    keys: Vec<serde_json::Value>,
}

struct IssuerKeys {
    keys: HashMap<String, DecodingKey>,
    fetched_at: Instant,
}

struct Inner {
    accepted: AcceptedClients,
    audience: Option<String>,
    http: reqwest::Client,
    refresh_interval: Duration,
    cache: RwLock<HashMap<String, IssuerKeys>>,
}

#[derive(Clone)]
pub struct FerrisKeyVerifier {
    inner: Arc<Inner>,
}

impl FerrisKeyVerifier {
    pub fn new(accepted: AcceptedClients) -> Self {
        Self::build(accepted, None, DEFAULT_REFRESH_INTERVAL)
    }

    pub fn with_refresh_interval(accepted: AcceptedClients, refresh_interval: Duration) -> Self {
        Self::build(accepted, None, refresh_interval)
    }

    /// Also requires `aud` to contain `audience`: the resource indicator the
    /// console asks for, or the client id a token was exchanged for.
    pub fn with_audience(accepted: AcceptedClients, audience: impl Into<String>) -> Self {
        Self::build(accepted, Some(audience.into()), DEFAULT_REFRESH_INTERVAL)
    }

    fn build(
        accepted: AcceptedClients,
        audience: Option<String>,
        refresh_interval: Duration,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            inner: Arc::new(Inner {
                accepted,
                audience,
                http,
                refresh_interval,
                cache: RwLock::new(HashMap::new()),
            }),
        }
    }

    async fn decoding_key(&self, issuer: &str, kid: &str) -> Result<DecodingKey, AuthError> {
        {
            let cache = self.inner.cache.read().await;
            if let Some(entry) = cache.get(issuer) {
                {
                    if let Some(key) = entry.keys.get(kid) {
                        return Ok(key.clone());
                    }
                    if entry.fetched_at.elapsed() < self.inner.refresh_interval {
                        return Err(AuthError::KeyNotFound(kid.to_owned()));
                    }
                }
            }
        }

        let keys = self.fetch_keys(issuer).await?;
        let found = keys.get(kid).cloned();
        self.inner.cache.write().await.insert(
            issuer.to_owned(),
            IssuerKeys {
                keys,
                fetched_at: Instant::now(),
            },
        );
        found.ok_or_else(|| AuthError::KeyNotFound(kid.to_owned()))
    }

    async fn fetch_keys(&self, issuer: &str) -> Result<HashMap<String, DecodingKey>, AuthError> {
        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        let discovery: Discovery = self.get_json(&discovery_url).await?;
        if discovery.issuer != issuer {
            return Err(AuthError::Discovery(format!(
                "discovery issuer mismatch: expected {issuer}, got {}",
                discovery.issuer
            )));
        }
        let jwks: RawJwks = self.get_json(&discovery.jwks_uri).await?;
        Ok(jwks.keys.into_iter().filter_map(decoding_entry).collect())
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, AuthError> {
        self.inner
            .http
            .get(url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| AuthError::Discovery(e.without_url().to_string()))?
            .json::<T>()
            .await
            .map_err(|e| AuthError::Discovery(e.without_url().to_string()))
    }

    fn check_client(&self, claims: &Claims) -> Result<(), AuthError> {
        let azp_accepted = claims
            .azp
            .as_deref()
            .is_some_and(|azp| self.inner.accepted.accepts(azp));
        let actor_accepted = claims
            .act
            .as_ref()
            .and_then(|actor| actor.client_id.as_deref())
            .is_some_and(|id| self.inner.accepted.accepts(id));
        if azp_accepted || actor_accepted {
            return Ok(());
        }
        Err(AuthError::UntrustedClient(
            claims.azp.clone().unwrap_or_default(),
        ))
    }
}

fn decoding_entry(value: serde_json::Value) -> Option<(String, DecodingKey)> {
    let jwk: Jwk = serde_json::from_value(value).ok()?;
    if !matches!(jwk.algorithm, AlgorithmParameters::RSA(_)) {
        return None;
    }
    if let Some(usage) = &jwk.common.public_key_use
        && *usage != PublicKeyUse::Signature
    {
        return None;
    }
    let kid = jwk.common.key_id.clone()?;
    let key = DecodingKey::from_jwk(&jwk).ok()?;
    Some((kid, key))
}

impl TokenVerifier for FerrisKeyVerifier {
    #[tracing::instrument(skip_all, err)]
    async fn verify(&self, token: &str, issuer: &str) -> Result<Identity, AuthError> {
        let header =
            jsonwebtoken::decode_header(token).map_err(|e| AuthError::Malformed(e.to_string()))?;
        if header.alg != Algorithm::RS256 {
            return Err(AuthError::InvalidToken(format!(
                "unsupported algorithm {:?}",
                header.alg
            )));
        }
        let kid = header
            .kid
            .ok_or_else(|| AuthError::Malformed("missing kid header".to_owned()))?;

        let key = self.decoding_key(issuer, &kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer]);
        validation.validate_exp = true;
        validation.leeway = EXPIRY_LEEWAY_SECS;
        match &self.inner.audience {
            Some(audience) => {
                validation.set_audience(&[audience]);
                validation.set_required_spec_claims(&["exp", "iss", "sub", "aud"]);
            }
            None => {
                validation.validate_aud = false;
                validation.set_required_spec_claims(&["exp", "iss", "sub"]);
            }
        }

        let data = jsonwebtoken::decode::<Claims>(token, &key, &validation).map_err(|e| match e
            .kind()
        {
            ErrorKind::ExpiredSignature => AuthError::Expired,
            _ => AuthError::InvalidToken(e.to_string()),
        })?;
        let claims = data.claims;

        if let Some(typ) = claims.typ.as_deref()
            && typ != BEARER_TYP
        {
            return Err(AuthError::InvalidToken(format!(
                "unexpected token typ {typ}"
            )));
        }

        self.check_client(&claims)?;
        Identity::try_from(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_with_payload(payload: &str) -> String {
        fn b64(input: &[u8]) -> String {
            const T: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
            let mut out = String::new();
            for chunk in input.chunks(3) {
                let n = chunk
                    .iter()
                    .enumerate()
                    .fold(0u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
                for i in 0..=chunk.len() {
                    out.push(char::from(T[((n >> (18 - 6 * i)) & 63) as usize]));
                }
            }
            out
        }
        format!(
            "{}.{}.sig",
            b64(br#"{"alg":"RS256","typ":"JWT"}"#),
            b64(payload.as_bytes())
        )
    }

    #[test]
    fn reads_issuer_without_verification() {
        let token = token_with_payload(r#"{"iss":"http://idp/realms/acme","sub":"x"}"#);
        assert_eq!(unverified_issuer(&token).unwrap(), "http://idp/realms/acme");
    }

    #[test]
    fn garbage_is_malformed() {
        assert!(matches!(
            unverified_issuer("not-a-token"),
            Err(AuthError::Malformed(_))
        ));
        assert!(matches!(
            unverified_issuer(""),
            Err(AuthError::Malformed(_))
        ));
    }
}
