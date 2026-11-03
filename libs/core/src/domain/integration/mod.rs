pub mod ports;
pub mod service;

use capitainerie_common::{ClientId, CoreError, IntegrationInstanceId, OrganizationId};
use chrono::{DateTime, SubsecRound, Utc};

use crate::domain::{agent::SecretToken, label::clean_label};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegrationKind {
    OdooJson2,
    OdooJsonRpc,
    N8n,
}

impl IntegrationKind {
    pub const ALL: [Self; 3] = [Self::OdooJson2, Self::OdooJsonRpc, Self::N8n];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OdooJson2 => "odoo_json2",
            Self::OdooJsonRpc => "odoo_jsonrpc",
            Self::N8n => "n8n",
        }
    }

    pub fn requires_database_and_login(self) -> bool {
        matches!(self, Self::OdooJsonRpc)
    }

    pub fn is_odoo(self) -> bool {
        matches!(self, Self::OdooJson2 | Self::OdooJsonRpc)
    }
}

impl TryFrom<&str> for IntegrationKind {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown integration kind {value}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Environment {
    Production,
    Staging,
    Development,
}

impl Environment {
    pub const ALL: [Self; 3] = [Self::Production, Self::Staging, Self::Development];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Staging => "staging",
            Self::Development => "development",
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

impl TryFrom<&str> for Environment {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|environment| environment.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown environment {value}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceName(String);

impl InstanceName {
    pub const MAX_CHARS: usize = 80;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for InstanceName {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        clean_label(&value, Self::MAX_CHARS, "the instance name").map(Self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUrl {
    url: String,
    host_start: usize,
    host_end: usize,
}

impl BaseUrl {
    pub const MAX_CHARS: usize = 2048;

    pub fn as_str(&self) -> &str {
        &self.url
    }

    pub fn is_https(&self) -> bool {
        self.url.starts_with("https://")
    }

    pub fn host(&self) -> &str {
        &self.url[self.host_start..self.host_end]
    }

    pub fn join(&self, path: &str) -> String {
        format!("{}/{}", self.url, path.trim_start_matches('/'))
    }
}

impl TryFrom<String> for BaseUrl {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let invalid = |why: &str| CoreError::Invalid(format!("the URL {why}"));
        let trimmed = value.trim();
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(invalid(&format!("exceeds {} characters", Self::MAX_CHARS)));
        }
        if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(invalid("contains a space or a control character"));
        }
        let lowered = trimmed.to_ascii_lowercase();
        let scheme_len = if lowered.starts_with("https://") {
            "https://".len()
        } else if lowered.starts_with("http://") {
            "http://".len()
        } else {
            return Err(invalid("must start with https:// or http://"));
        };
        if trimmed.contains(['?', '#']) {
            return Err(invalid("must not carry a query or a fragment"));
        }
        let rest = &trimmed[scheme_len..];
        let authority_len = rest.find('/').unwrap_or(rest.len());
        let authority = &rest[..authority_len];
        if authority.contains('@') {
            return Err(invalid("must not carry credentials"));
        }
        let host_len = if authority.starts_with('[') {
            authority
                .find(']')
                .map(|end| end + 1)
                .ok_or_else(|| invalid("has an unclosed IPv6 address"))?
        } else {
            authority.find(':').unwrap_or(authority.len())
        };
        let host = &authority[..host_len];
        if host.is_empty() || host == "[]" {
            return Err(invalid("has no host"));
        }
        let port = &authority[host_len..];
        if !port.is_empty() {
            let digits = port
                .strip_prefix(':')
                .ok_or_else(|| invalid("has an invalid port"))?;
            if digits.is_empty() || digits.parse::<u16>().is_err() {
                return Err(invalid("has an invalid port"));
            }
        }
        let url = format!("{}{}", &lowered[..scheme_len], rest.trim_end_matches('/'));
        let host_start = scheme_len;
        Ok(Self {
            host_end: host_start + host_len,
            host_start,
            url,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationInstance {
    pub id: IntegrationInstanceId,
    pub organization_id: OrganizationId,
    pub client_id: ClientId,
    pub kind: IntegrationKind,
    pub name: InstanceName,
    pub environment: Environment,
    pub base_url: BaseUrl,
    pub database: Option<String>,
    pub login: Option<String>,
    pub secret_preview: String,
    pub server_version: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl IntegrationInstance {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn connection(&self, secret: SecretToken) -> InstanceConnection {
        InstanceConnection {
            instance_id: self.id,
            kind: self.kind,
            base_url: self.base_url.clone(),
            database: self.database.clone(),
            login: self.login.clone(),
            secret,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceConnection {
    pub instance_id: IntegrationInstanceId,
    pub kind: IntegrationKind,
    pub base_url: BaseUrl,
    pub database: Option<String>,
    pub login: Option<String>,
    pub secret: SecretToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub server_version: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConnectorFailure {
    #[error("the network policy blocks this address: {0}")]
    Blocked(String),
    #[error("the instance cannot be reached: {0}")]
    Unreachable(String),
    #[error("the instance did not answer in time")]
    Timeout,
    #[error("the instance refused the credentials")]
    Unauthorized,
    #[error("the instance does not serve this protocol: {0}")]
    VersionMismatch(String),
    #[error("the instance answered something unexpected: {0}")]
    Protocol(String),
    #[error("this instance does not support it: {0}")]
    NotSupported(String),
    #[error("the instance answered {status}: {message}")]
    Remote { status: u16, message: String },
}

impl ConnectorFailure {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Blocked(_) => "network_blocked",
            Self::VersionMismatch(_) => "version_mismatch",
            Self::NotSupported(_) => "not_supported",
            Self::Unreachable(_)
            | Self::Timeout
            | Self::Unauthorized
            | Self::Protocol(_)
            | Self::Remote { .. } => "connection_failed",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Connector(#[from] ConnectorFailure),
    #[error("this client already has an instance named {0}")]
    NameTaken(String),
}

pub fn stamp() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(6)
}

pub fn secret_preview(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() < 12 {
        return "…".to_owned();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("…{tail}")
}

pub fn clean_odoo_field(value: String, max: usize, what: &str) -> Result<String, CoreError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(CoreError::Invalid(format!("{what} is empty")));
    }
    if value.chars().count() > max {
        return Err(CoreError::Invalid(format!(
            "{what} exceeds {max} characters"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(CoreError::Invalid(format!(
            "{what} contains a control character"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(raw: &str) -> Result<BaseUrl, CoreError> {
        BaseUrl::try_from(raw.to_owned())
    }

    #[test]
    fn a_url_keeps_its_path_and_loses_its_trailing_slash() {
        let parsed = url("HTTPS://odoo.acme.test/erp/").unwrap();
        assert_eq!(parsed.as_str(), "https://odoo.acme.test/erp");
        assert_eq!(parsed.host(), "odoo.acme.test");
        assert!(parsed.is_https());
        assert_eq!(
            parsed.join("/web/version"),
            "https://odoo.acme.test/erp/web/version"
        );
    }

    #[test]
    fn a_url_with_a_port_or_an_ipv6_host_is_parsed() {
        assert_eq!(url("http://n8n.local:5678").unwrap().host(), "n8n.local");
        assert_eq!(url("https://[::1]:8069/").unwrap().host(), "[::1]");
    }

    #[test]
    fn a_url_with_credentials_a_query_or_a_bad_scheme_is_refused() {
        for raw in [
            "ftp://odoo.acme.test",
            "odoo.acme.test",
            "https://user:pass@odoo.acme.test",
            "https://odoo.acme.test/?db=prod",
            "https://odoo.acme.test/#x",
            "https://",
            "https://odoo.acme.test:99999",
            "https://odoo.acme.test:",
            "https://odoo acme.test",
        ] {
            assert!(url(raw).is_err(), "{raw} should be refused");
        }
    }

    #[test]
    fn kinds_and_environments_round_trip() {
        for kind in IntegrationKind::ALL {
            assert_eq!(IntegrationKind::try_from(kind.as_str()).unwrap(), kind);
        }
        for environment in Environment::ALL {
            assert_eq!(
                Environment::try_from(environment.as_str()).unwrap(),
                environment
            );
        }
        assert!(IntegrationKind::try_from("odoo").is_err());
        assert!(IntegrationKind::OdooJsonRpc.requires_database_and_login());
        assert!(!IntegrationKind::OdooJson2.requires_database_and_login());
    }

    #[test]
    fn a_preview_shows_only_the_last_four_characters_of_a_long_secret() {
        assert_eq!(secret_preview("sk-0123456789abcd"), "…abcd");
        assert_eq!(secret_preview("short"), "…");
    }

    #[test]
    fn a_connector_failure_has_a_stable_code() {
        assert_eq!(
            ConnectorFailure::Blocked("10.0.0.5".into()).code(),
            "network_blocked"
        );
        assert_eq!(ConnectorFailure::Unauthorized.code(), "connection_failed");
        assert_eq!(
            ConnectorFailure::VersionMismatch("no JSON-2".into()).code(),
            "version_mismatch"
        );
    }
}
