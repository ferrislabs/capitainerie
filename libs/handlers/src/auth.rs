use axum::{
    extract::{Request, State},
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use capitainerie_auth::{Identity, TokenVerifier, unverified_issuer};
use capitainerie_common::CoreError;
use capitainerie_core::{
    Organization,
    domain::{agent::SecretToken, authorization::Subject},
};

use crate::{ApiError, AppState};

/// The realm role FerrisKey gives the admins of an organization.
pub const ADMIN_ROLE: &str = "capitainerie-admin";

/// Who is calling, inserted as a request extension by [`authenticate`].
#[derive(Debug, Clone)]
pub struct Caller {
    pub identity: Identity,
    pub organization: Organization,
    /// The caller's own bearer token, to exchange for an agent's on their behalf.
    pub token: SecretToken,
}

impl Caller {
    /// The caller as the policy decision point sees it.
    pub fn subject(&self) -> Subject {
        Subject {
            id: self.identity.subject.clone(),
            organization_id: self.organization.id,
            actor: self
                .identity
                .actor
                .as_ref()
                .and_then(|actor| actor.client_id.clone()),
            is_admin: self.identity.roles.iter().any(|role| role == ADMIN_ROLE),
        }
    }
}

/// The realm that issued the token must belong to an organization before its
/// signature is even checked: no key is ever fetched from an unknown issuer.
pub async fn authenticate(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = bearer_token(request.headers())?.to_owned();
    let issuer = unverified_issuer(&token)?;
    let organization = state
        .usecase
        .organization_by_issuer(&issuer)
        .await
        .map_err(|error| match error {
            CoreError::NotFound => ApiError::Unauthorized("unknown issuer".into()),
            other => other.into(),
        })?;
    let identity = state
        .verifier
        .verify(&token, &organization.issuer_url)
        .await?;

    request.extensions_mut().insert(Caller {
        identity,
        organization,
        token: SecretToken::new(token),
    });
    Ok(next.run(request).await)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ApiError::Unauthorized("missing bearer token".into()))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn reads_the_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer abc"));
        assert_eq!(bearer_token(&headers).unwrap(), "abc");
    }

    #[test]
    fn rejects_missing_or_other_schemes() {
        let mut headers = HeaderMap::new();
        assert!(bearer_token(&headers).is_err());
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert!(bearer_token(&headers).is_err());
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer "));
        assert!(bearer_token(&headers).is_err());
    }
}
