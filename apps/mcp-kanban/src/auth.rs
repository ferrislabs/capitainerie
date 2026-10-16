use std::sync::Arc;

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use capitainerie_auth::{AuthError, Identity, TokenVerifier, unverified_issuer};
use capitainerie_common::CoreError;
use capitainerie_core::{CapitainerieUseCase, domain::authorization::Subject};
use serde_json::{Value, json};

pub struct AuthState<V> {
    pub usecase: CapitainerieUseCase,
    pub verifier: V,
    pub public_url: String,
}

impl<V> AuthState<V> {
    fn metadata_url(&self) -> String {
        metadata_url(&self.public_url)
    }
}

pub fn metadata_url(public_url: &str) -> String {
    format!(
        "{}/.well-known/oauth-protected-resource",
        public_url.trim_end_matches('/')
    )
}

pub fn protected_resource(public_url: &str) -> Value {
    json!({
        "resource": public_url.trim_end_matches('/'),
        "bearer_methods_supported": ["header"],
        "scopes_supported": [],
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    MissingToken,
    InvalidToken(String),
    DelegationRequired,
    Unavailable,
    Internal,
}

impl Rejection {
    fn status(&self) -> StatusCode {
        match self {
            Self::MissingToken | Self::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            Self::DelegationRequired => StatusCode::FORBIDDEN,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn challenge(&self, metadata_url: &str) -> String {
        match self {
            Self::InvalidToken(_) => {
                format!(r#"Bearer error="invalid_token", resource_metadata="{metadata_url}""#)
            }
            _ => format!(r#"Bearer resource_metadata="{metadata_url}""#),
        }
    }

    fn into_response_with(self, metadata_url: &str) -> Response {
        let status = self.status();
        let mut response = match &self {
            Self::DelegationRequired => {
                (status, Json(json!({"error": "delegation_required"}))).into_response()
            }
            Self::InvalidToken(reason) => {
                tracing::info!(%reason, "token rejected");
                status.into_response()
            }
            _ => status.into_response(),
        };
        if status == StatusCode::UNAUTHORIZED
            && let Ok(value) = HeaderValue::from_str(&self.challenge(metadata_url))
        {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, value);
        }
        response
    }
}

impl From<AuthError> for Rejection {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::Discovery(_) => Self::Unavailable,
            other => Self::InvalidToken(other.to_string()),
        }
    }
}

pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

pub fn delegated_subject(
    identity: &Identity,
    organization_id: capitainerie_common::OrganizationId,
) -> Result<Subject, Rejection> {
    let agent = identity
        .actor
        .as_ref()
        .and_then(|actor| actor.client_id.clone())
        .ok_or(Rejection::DelegationRequired)?;
    Ok(Subject {
        id: identity.subject.clone(),
        organization_id,
        actor: Some(agent),
    })
}

pub async fn authenticate<V: TokenVerifier>(
    State(state): State<Arc<AuthState<V>>>,
    mut request: Request,
    next: Next,
) -> Response {
    match subject_of(&state, request.headers()).await {
        Ok(subject) => {
            request.extensions_mut().insert(subject);
            next.run(request).await
        }
        Err(rejection) => rejection.into_response_with(&state.metadata_url()),
    }
}

async fn subject_of<V: TokenVerifier>(
    state: &AuthState<V>,
    headers: &HeaderMap,
) -> Result<Subject, Rejection> {
    let token = bearer_token(headers).ok_or(Rejection::MissingToken)?;
    let issuer = unverified_issuer(token)?;
    let organization = state
        .usecase
        .organization_by_issuer(&issuer)
        .await
        .map_err(|error| match error {
            CoreError::NotFound => Rejection::InvalidToken("unknown issuer".into()),
            CoreError::Database(_) => Rejection::Unavailable,
            _ => Rejection::Internal,
        })?;
    let identity = state
        .verifier
        .verify(token, &organization.issuer_url)
        .await?;
    delegated_subject(&identity, organization.id)
}

#[cfg(test)]
mod tests {
    use capitainerie_auth::Actor;
    use capitainerie_common::OrganizationId;
    use uuid::Uuid;

    use super::*;

    fn identity(actor: Option<Actor>) -> Identity {
        Identity {
            subject: "user-1".into(),
            issuer: "http://idp/realms/r".into(),
            client_id: "capitainerie-agent-writer".into(),
            username: None,
            email: None,
            display_name: None,
            actor,
        }
    }

    #[test]
    fn reads_the_bearer_token() {
        let mut headers = HeaderMap::new();
        assert_eq!(bearer_token(&headers), None);
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Basic x"));
        assert_eq!(bearer_token(&headers), None);
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));
        assert_eq!(bearer_token(&headers), None);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer abc"),
        );
        assert_eq!(bearer_token(&headers), Some("abc"));
    }

    #[test]
    fn a_delegated_identity_becomes_a_subject_with_the_agent_as_actor() {
        let organization = OrganizationId(Uuid::now_v7());
        let actor = Actor {
            sub: "service-account".into(),
            client_id: Some("capitainerie-agent-writer".into()),
            act: None,
        };
        let subject = delegated_subject(&identity(Some(actor)), organization).unwrap();
        assert_eq!(subject.id, "user-1");
        assert_eq!(subject.organization_id, organization);
        assert_eq!(subject.actor.as_deref(), Some("capitainerie-agent-writer"));
    }

    #[test]
    fn a_token_without_delegation_is_refused() {
        let organization = OrganizationId(Uuid::now_v7());
        assert_eq!(
            delegated_subject(&identity(None), organization),
            Err(Rejection::DelegationRequired)
        );
        let no_client = Actor {
            sub: "service-account".into(),
            client_id: None,
            act: None,
        };
        assert_eq!(
            delegated_subject(&identity(Some(no_client)), organization),
            Err(Rejection::DelegationRequired)
        );
    }

    #[test]
    fn discovery_failures_are_unavailable_and_others_unauthorized() {
        assert_eq!(
            Rejection::from(AuthError::Discovery("down".into())),
            Rejection::Unavailable
        );
        assert_eq!(
            Rejection::from(AuthError::Expired).status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn metadata_describes_the_resource() {
        assert_eq!(
            metadata_url("http://localhost:4101/"),
            "http://localhost:4101/.well-known/oauth-protected-resource"
        );
        let document = protected_resource("http://localhost:4101/");
        assert_eq!(document["resource"], "http://localhost:4101");
        assert_eq!(document["bearer_methods_supported"][0], "header");
    }
}
