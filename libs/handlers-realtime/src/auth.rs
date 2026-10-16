use std::future::Future;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use capitainerie_auth::{TokenVerifier, unverified_issuer};
use capitainerie_handlers::AppState;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub subject: String,
    pub organization_id: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Denied(pub &'static str);

pub trait Authenticate: Clone + Send + Sync + 'static {
    fn authenticate(&self, token: &str) -> impl Future<Output = Result<Session, Denied>> + Send;
}

impl Authenticate for AppState {
    async fn authenticate(&self, token: &str) -> Result<Session, Denied> {
        let issuer = unverified_issuer(token).map_err(|error| {
            tracing::warn!(%error, "realtime: malformed token");
            Denied("unauthorized")
        })?;
        let organization = self
            .usecase
            .organization_by_issuer(&issuer)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "realtime: unknown issuer");
                Denied("unauthorized")
            })?;
        let identity = self
            .verifier
            .verify(token, &organization.issuer_url)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "realtime: token refused");
                Denied("unauthorized")
            })?;
        Ok(Session {
            subject: identity.subject,
            organization_id: organization.id.0,
            expires_at: expiry(token),
        })
    }
}

#[derive(Deserialize)]
struct ExpOnly {
    exp: i64,
}

/// Reads `exp` from a token the verifier has already accepted.
fn expiry(token: &str) -> Option<DateTime<Utc>> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims: ExpOnly = serde_json::from_slice(&bytes).ok()?;
    DateTime::from_timestamp(claims.exp, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(payload: &str) -> String {
        format!("h.{}.s", URL_SAFE_NO_PAD.encode(payload))
    }

    #[test]
    fn reads_exp_from_the_payload() {
        let at = expiry(&token(r#"{"exp":1800000000,"sub":"u"}"#)).unwrap();
        assert_eq!(at.timestamp(), 1_800_000_000);
    }

    #[test]
    fn missing_or_malformed_exp_is_none() {
        assert!(expiry(&token(r#"{"sub":"u"}"#)).is_none());
        assert!(expiry("garbage").is_none());
        assert!(expiry("a.!!!.c").is_none());
    }
}
