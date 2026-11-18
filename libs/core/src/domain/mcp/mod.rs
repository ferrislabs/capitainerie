pub mod pkce;
pub mod ports;
pub mod service;

use capitainerie_common::{CoreError, McpConnectorId, OrganizationId};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use url::Url;

use crate::domain::{agent::SecretToken, crew::Slug};

pub const CALLBACK_PATH: &str = "/connectors/oauth/callback";
pub const REFRESH_MARGIN_SECONDS: i64 = 60;
pub const STATE_TTL_MINUTES: i64 = 10;
pub const URL_MAX_CHARS: usize = 2_048;
pub const SCOPES_MAX_COUNT: usize = 100;
pub const TOOL_TEXT_MAX_CHARS: usize = 12_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnector {
    pub id: McpConnectorId,
    pub organization_id: OrganizationId,
    pub slug: Slug,
    pub name: String,
    pub url: String,
    pub resource: String,
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub archived: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    pub resource: String,
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    pub access_token: SecretToken,
    pub refresh_token: Option<SecretToken>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

impl TokenSet {
    pub fn needs_refresh(&self, now: DateTime<Utc>) -> bool {
        self.expires_at
            .is_some_and(|at| at - now < Duration::seconds(REFRESH_MARGIN_SECONDS))
    }

    pub fn renewed_by(&self, fresh: TokenSet) -> TokenSet {
        TokenSet {
            access_token: fresh.access_token,
            refresh_token: fresh.refresh_token.or_else(|| self.refresh_token.clone()),
            expires_at: fresh.expires_at,
            scope: fresh.scope.or_else(|| self.scope.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpConnection {
    pub connector_id: McpConnectorId,
    pub subject: String,
    pub tokens: TokenSet,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionStatus {
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorView {
    pub connector: McpConnector,
    pub connection: Option<ConnectionStatus>,
}

#[derive(Debug, Clone)]
pub struct PendingAuthorization {
    pub state: String,
    pub connector_id: McpConnectorId,
    pub subject: String,
    pub code_verifier: SecretToken,
    pub redirect_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationStart {
    pub authorization_url: String,
    pub state: String,
}

pub struct CodeExchange<'a> {
    pub token_endpoint: &'a str,
    pub client_id: &'a str,
    pub resource: &'a str,
    pub code: &'a str,
    pub code_verifier: &'a SecretToken,
    pub redirect_uri: &'a str,
}

pub struct RefreshExchange<'a> {
    pub token_endpoint: &'a str,
    pub client_id: &'a str,
    pub resource: &'a str,
    pub refresh_token: &'a SecretToken,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
    pub read_only_hint: Option<bool>,
    pub destructive_hint: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    pub text: String,
    pub is_error: bool,
}

impl PendingAuthorization {
    pub fn begin(
        connector: &McpConnector,
        subject: &str,
        redirect_uri: &str,
        now: DateTime<Utc>,
    ) -> Result<(Self, String), CoreError> {
        let pkce = pkce::Pkce::generate()?;
        let pending = Self {
            state: pkce::random_state()?,
            connector_id: connector.id,
            subject: subject.to_owned(),
            code_verifier: SecretToken::new(pkce.verifier),
            redirect_uri: redirect_uri.to_owned(),
            expires_at: now + Duration::minutes(STATE_TTL_MINUTES),
        };
        Ok((pending, pkce.challenge))
    }

    pub fn check(&self, subject: &str, now: DateTime<Utc>) -> Result<(), CoreError> {
        if self.subject != subject {
            return Err(CoreError::Forbidden {
                reason: Some("cette autorisation a été demandée par un autre utilisateur".into()),
            });
        }
        if self.expires_at <= now {
            return Err(CoreError::Invalid(
                "l'autorisation a expiré, recommencez la connexion".into(),
            ));
        }
        Ok(())
    }
}

pub fn authorization_url(
    connector: &McpConnector,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> Result<String, CoreError> {
    let mut url = Url::parse(&connector.authorization_endpoint)
        .map_err(|_| CoreError::Internal("a stored authorization endpoint is not a URL".into()))?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("response_type", "code")
            .append_pair("client_id", &connector.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("resource", &connector.resource);
        if !connector.scopes.is_empty() {
            query.append_pair("scope", &connector.scopes.join(" "));
        }
    }
    Ok(url.into())
}

pub fn callback_url(console_url: &str) -> String {
    format!("{}{CALLBACK_PATH}", console_url.trim_end_matches('/'))
}

pub fn parse_server_url(value: &str, what: &str) -> Result<Url, CoreError> {
    let invalid = || CoreError::Invalid(format!("{what} n'est pas une URL http(s) valide"));
    let value = value.trim();
    if value.chars().count() > URL_MAX_CHARS {
        return Err(invalid());
    }
    let url = Url::parse(value).map_err(|_| invalid())?;
    let acceptable = matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none();
    if acceptable { Ok(url) } else { Err(invalid()) }
}

pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug
        .trim_matches('-')
        .chars()
        .take(Slug::MAX_CHARS)
        .collect();
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "connecteur".to_owned()
    } else {
        slug.to_owned()
    }
}

pub fn cap_text(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn connector(organization_id: OrganizationId, slug: &str) -> McpConnector {
        let now = Utc::now();
        McpConnector {
            id: McpConnectorId::new(),
            organization_id,
            slug: Slug::try_from(slug.to_owned()).unwrap(),
            name: slug.to_owned(),
            url: "https://demo.mestier.fr/mcp".into(),
            resource: "https://demo.mestier.fr/mcp".into(),
            issuer: "https://auth.example/realms/mestier".into(),
            authorization_endpoint: "https://auth.example/realms/mestier/auth".into(),
            token_endpoint: "https://auth.example/realms/mestier/token".into(),
            client_id: "client-1".into(),
            scopes: vec!["mestier.read".into(), "mestier.doe.write".into()],
            archived: false,
            created_by: "user-1".into(),
            created_at: now,
            updated_at: now,
        }
    }

    fn tokens(expires_in: Option<i64>) -> TokenSet {
        TokenSet {
            access_token: SecretToken::new("access"),
            refresh_token: Some(SecretToken::new("refresh")),
            expires_at: expires_in.map(|seconds| Utc::now() + Duration::seconds(seconds)),
            scope: Some("mestier.read".into()),
        }
    }

    #[test]
    fn a_token_is_refreshed_one_minute_before_it_expires() {
        let now = Utc::now();
        assert!(!tokens(None).needs_refresh(now));
        assert!(!tokens(Some(3_600)).needs_refresh(now));
        assert!(!tokens(Some(120)).needs_refresh(now));
        assert!(tokens(Some(59)).needs_refresh(now));
        assert!(tokens(Some(-5)).needs_refresh(now));
    }

    #[test]
    fn a_renewal_keeps_what_the_server_did_not_send_again() {
        let old = tokens(Some(-5));
        let kept = old.renewed_by(TokenSet {
            access_token: SecretToken::new("new-access"),
            refresh_token: None,
            expires_at: None,
            scope: None,
        });
        assert_eq!(kept.access_token.expose(), "new-access");
        assert_eq!(kept.refresh_token.unwrap().expose(), "refresh");
        assert_eq!(kept.scope.as_deref(), Some("mestier.read"));
        assert_eq!(kept.expires_at, None);

        let rotated = old.renewed_by(TokenSet {
            access_token: SecretToken::new("a2"),
            refresh_token: Some(SecretToken::new("r2")),
            expires_at: None,
            scope: Some("other".into()),
        });
        assert_eq!(rotated.refresh_token.unwrap().expose(), "r2");
        assert_eq!(rotated.scope.as_deref(), Some("other"));
    }

    #[test]
    fn a_state_is_good_for_its_subject_and_ten_minutes() {
        let connector = connector(OrganizationId::new(), "mestier");
        let now = Utc::now();
        let (pending, challenge) =
            PendingAuthorization::begin(&connector, "user-1", "https://console/cb", now).unwrap();

        assert_eq!(pending.expires_at, now + Duration::minutes(10));
        assert_eq!(
            challenge,
            pkce::challenge_of(pending.code_verifier.expose())
        );
        assert!(pending.check("user-1", now).is_ok());
        assert!(
            pending
                .check("user-1", now + Duration::minutes(10) - Duration::seconds(1))
                .is_ok()
        );
        assert!(matches!(
            pending.check("user-1", now + Duration::minutes(10)),
            Err(CoreError::Invalid(_))
        ));
        assert!(matches!(
            pending.check("user-2", now),
            Err(CoreError::Forbidden { .. })
        ));
    }

    #[test]
    fn the_authorization_url_carries_pkce_scope_and_resource() {
        let connector = connector(OrganizationId::new(), "mestier");
        let url = authorization_url(&connector, "https://console/cb", "st", "ch").unwrap();
        let parsed = Url::parse(&url).unwrap();
        let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(parsed.path(), "/realms/mestier/auth");
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["client_id"], "client-1");
        assert_eq!(query["redirect_uri"], "https://console/cb");
        assert_eq!(query["state"], "st");
        assert_eq!(query["code_challenge"], "ch");
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(query["resource"], "https://demo.mestier.fr/mcp");
        assert_eq!(query["scope"], "mestier.read mestier.doe.write");
    }

    #[test]
    fn server_urls_are_http_without_credentials() {
        for good in ["https://demo.mestier.fr/mcp", "http://localhost:3000/mcp"] {
            assert!(parse_server_url(good, "l'URL").is_ok(), "{good}");
        }
        for bad in [
            "",
            "ftp://host/mcp",
            "https://user:pw@host/mcp",
            "https://host/mcp#frag",
            "not a url",
            "javascript:alert(1)",
        ] {
            assert!(parse_server_url(bad, "l'URL").is_err(), "{bad}");
        }
    }

    #[test]
    fn slugs_are_derived_from_names() {
        assert_eq!(slugify("Mestier — Démo"), "mestier-d-mo");
        assert_eq!(slugify("  ERP 2  "), "erp-2");
        assert_eq!(slugify("!!!"), "connecteur");
        assert!(Slug::try_from(slugify(&"a".repeat(100))).is_ok());
    }

    #[test]
    fn long_tool_output_is_cut_on_a_character_boundary() {
        assert_eq!(cap_text("abc", 3), "abc");
        assert_eq!(cap_text("abcd", 3), "abc…");
        assert_eq!(cap_text("ééééé", 2), "éé…");
    }
}
