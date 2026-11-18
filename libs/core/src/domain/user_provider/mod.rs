pub mod ports;
pub mod service;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use capitainerie_common::{CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use crate::domain::{
    agent::SecretToken,
    integration::BaseUrl,
    provider::{ModelRef, ProviderAccess, ProviderKind, model_ref_of, service::validate_provider},
};

pub const NO_PROVIDER_MESSAGE: &str =
    "Aucun fournisseur LLM connecté : ajoutez votre clé dans Paramètres > Fournisseurs";
const MODEL_MAX_CHARS: usize = 200;
const LOCAL_SUFFIXES: [&str; 6] = [
    ".localhost",
    ".local",
    ".internal",
    ".localdomain",
    ".lan",
    ".home.arpa",
];

/// A user's own key for one provider. The secret never leaves the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProviderCredential {
    pub organization_id: OrganizationId,
    pub subject: String,
    pub kind: ProviderKind,
    pub secret: SecretToken,
    pub base_url: Option<String>,
    pub default_model: String,
    pub preview: String,
    pub is_default: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProviderSummary {
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub default_model: String,
    pub preview: String,
    pub is_default: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<&UserProviderCredential> for UserProviderSummary {
    fn from(credential: &UserProviderCredential) -> Self {
        Self {
            kind: credential.kind,
            base_url: credential.base_url.clone(),
            default_model: credential.default_model.clone(),
            preview: credential.preview.clone(),
            is_default: credential.is_default,
            updated_at: credential.updated_at,
        }
    }
}

impl From<UserProviderCredential> for ProviderAccess {
    fn from(credential: UserProviderCredential) -> Self {
        Self {
            kind: credential.kind,
            secret: credential.secret,
            base_url: credential.base_url,
        }
    }
}

pub fn validate_default_model(model: &str) -> Result<String, CoreError> {
    let model = model.trim();
    if model.is_empty() || model.chars().count() > MODEL_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "default_model must hold 1 to {MODEL_MAX_CHARS} characters"
        )));
    }
    if !crate::domain::provider::is_model_id(model) {
        return Err(CoreError::Invalid(
            "default_model must be a model id such as claude-sonnet-5-5".into(),
        ));
    }
    Ok(model.to_owned())
}

/// A user brings the endpoint, so beyond the organization's rule it must be
/// https and name a public host: a literal private address or an internal
/// name would turn the worker into a proxy into the cluster.
pub fn validate_user_provider(
    kind: ProviderKind,
    secret: &SecretToken,
    base_url: Option<String>,
) -> Result<Option<String>, CoreError> {
    let kept = validate_provider(kind, secret, base_url)?;
    if let Some(url) = &kept {
        check_public_host(url)?;
    }
    Ok(kept)
}

fn check_public_host(url: &str) -> Result<(), CoreError> {
    let refused = |why: &str| CoreError::Invalid(format!("base_url {why}"));
    let base = BaseUrl::try_from(url.to_owned())?;
    if !base.is_https() {
        return Err(refused("must use https"));
    }
    let host = base
        .host()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    if let Ok(address) = host.parse::<IpAddr>() {
        return if is_refused(address) {
            Err(refused("must not point to a private or local address"))
        } else {
            Ok(())
        };
    }
    let last = host.rsplit('.').next().unwrap_or_default();
    let named = host.contains('.')
        && last.starts_with(|c: char| c.is_ascii_alphabetic())
        && !LOCAL_SUFFIXES.iter().any(|suffix| host.ends_with(suffix));
    if named {
        Ok(())
    } else {
        Err(refused("must name a public host"))
    }
}

fn is_refused(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_refused_v4(address),
        IpAddr::V6(address) => is_refused_v6(address),
    }
}

fn is_refused_v4(address: Ipv4Addr) -> bool {
    let [first, second, ..] = address.octets();
    address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_multicast()
        || address.is_unspecified()
        || first == 0
        || first >= 240
        || (first == 100 && second & 0xc0 == 64)
}

fn is_refused_v6(address: Ipv6Addr) -> bool {
    let first = address.segments()[0];
    address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || first & 0xfe00 == 0xfc00
        || first & 0xffc0 == 0xfe80
        || address.to_ipv4_mapped().is_some_and(is_refused_v4)
        || address.to_ipv4().is_some_and(is_refused_v4)
}

/// Which model serves a user's agent run, in order: the agent's own model on
/// a provider the user connected; the user's default connection; the
/// organization's model on a provider it holds a key for. `None` when none of
/// them applies.
pub fn choose_model(
    agent_model: &str,
    user: &[UserProviderSummary],
    organization: &[ProviderKind],
    organization_model: &ModelRef,
) -> Option<ModelRef> {
    let default = user.iter().find(|summary| summary.is_default);
    let user_kind = default.map_or(organization_model.provider, |summary| summary.kind);
    if let Some(wanted) = model_ref_of(agent_model, user_kind)
        && user.iter().any(|summary| summary.kind == wanted.provider)
    {
        return Some(wanted);
    }
    if let Some(default) = default {
        return Some(ModelRef {
            provider: default.kind,
            model: default.default_model.clone(),
        });
    }
    if let Some(wanted) = model_ref_of(agent_model, organization_model.provider)
        && organization.contains(&wanted.provider)
    {
        return Some(wanted);
    }
    organization
        .contains(&organization_model.provider)
        .then(|| organization_model.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(value: &str) -> SecretToken {
        SecretToken::new(value)
    }

    fn url(value: &str) -> Result<Option<String>, CoreError> {
        validate_user_provider(
            ProviderKind::OpenAiCompatible,
            &secret(""),
            Some(value.into()),
        )
    }

    #[test]
    fn a_public_https_endpoint_is_accepted_and_normalised() {
        let kept = url("https://generativelanguage.googleapis.com/v1beta/openai/").unwrap();
        assert_eq!(
            kept.as_deref(),
            Some("https://generativelanguage.googleapis.com/v1beta/openai")
        );
        assert!(url("https://8.8.8.8/v1").is_ok());
    }

    #[test]
    fn a_private_or_local_endpoint_is_refused() {
        for refused in [
            "http://api.example.com/v1",
            "https://localhost/v1",
            "https://127.0.0.1/v1",
            "https://10.1.2.3/v1",
            "https://192.168.0.1/v1",
            "https://169.254.169.254/latest",
            "https://100.64.0.1/v1",
            "https://[::1]/v1",
            "https://[fd00::1]/v1",
            "https://[::ffff:10.0.0.1]/v1",
            "https://ollama/v1",
            "https://ollama.internal/v1",
            "https://nas.local/v1",
            "https://0x7f.0.0.1/v1",
            "https://2130706433/v1",
            "https://user:pass@example.com/v1",
            "https://example.com/v1?x=1",
        ] {
            assert!(
                matches!(url(refused), Err(CoreError::Invalid(_))),
                "{refused}"
            );
        }
    }

    #[test]
    fn a_hosted_provider_ignores_the_base_url() {
        let kept = validate_user_provider(
            ProviderKind::Mistral,
            &secret("sk-key"),
            Some("https://127.0.0.1".into()),
        )
        .unwrap();
        assert_eq!(kept, None);
    }

    #[test]
    fn a_default_model_must_be_a_model_id() {
        assert_eq!(
            validate_default_model(" claude-sonnet-5-5 ").unwrap(),
            "claude-sonnet-5-5"
        );
        for refused in ["", "  ", "Claude Sonnet 5.5", "-x"] {
            assert!(validate_default_model(refused).is_err(), "{refused:?}");
        }
        assert!(validate_default_model(&"a".repeat(201)).is_err());
    }

    fn connection(kind: ProviderKind, model: &str, is_default: bool) -> UserProviderSummary {
        UserProviderSummary {
            kind,
            base_url: None,
            default_model: model.into(),
            preview: String::new(),
            is_default,
            updated_at: Utc::now(),
        }
    }

    fn organization_model() -> ModelRef {
        ModelRef {
            provider: ProviderKind::Mistral,
            model: "mistral-small".into(),
        }
    }

    fn user() -> Vec<UserProviderSummary> {
        vec![
            connection(ProviderKind::Anthropic, "claude-haiku-5-5", false),
            connection(ProviderKind::OpenAiCompatible, "gemini-3-flash", true),
        ]
    }

    fn chosen(
        agent_model: &str,
        user: &[UserProviderSummary],
        org: &[ProviderKind],
    ) -> Option<String> {
        choose_model(agent_model, user, org, &organization_model())
            .map(|model| format!("{}/{}", model.provider.as_str(), model.model))
    }

    #[test]
    fn the_agents_model_wins_on_a_provider_the_user_connected() {
        let org = [ProviderKind::Mistral];
        assert_eq!(
            chosen("claude-sonnet-5-5", &user(), &org).as_deref(),
            Some("anthropic/claude-sonnet-5-5")
        );
        assert_eq!(
            chosen("openai_compatible:gemini-3-pro", &user(), &org).as_deref(),
            Some("openai_compatible/gemini-3-pro")
        );
        assert_eq!(
            chosen("gemini-3-pro", &user(), &org).as_deref(),
            Some("openai_compatible/gemini-3-pro")
        );
    }

    #[test]
    fn a_model_on_an_unconnected_provider_falls_to_the_users_default() {
        let user = [connection(
            ProviderKind::OpenAiCompatible,
            "gemini-3-flash",
            true,
        )];
        assert_eq!(
            chosen("claude-sonnet-5-5", &user, &[ProviderKind::Anthropic]).as_deref(),
            Some("openai_compatible/gemini-3-flash")
        );
        assert_eq!(
            chosen("", &user, &[]).as_deref(),
            Some("openai_compatible/gemini-3-flash")
        );
    }

    #[test]
    fn without_a_user_connection_the_organization_serves() {
        assert_eq!(
            chosen("claude-sonnet-5-5", &[], &[ProviderKind::Anthropic]).as_deref(),
            Some("anthropic/claude-sonnet-5-5")
        );
        assert_eq!(
            chosen("", &[], &[ProviderKind::Mistral]).as_deref(),
            Some("mistral/mistral-small")
        );
        assert_eq!(
            chosen("claude-sonnet-5-5", &[], &[ProviderKind::Mistral]).as_deref(),
            Some("mistral/mistral-small")
        );
    }

    #[test]
    fn nothing_connected_anywhere_chooses_nothing() {
        assert_eq!(chosen("claude-sonnet-5-5", &[], &[]), None);
        assert_eq!(
            chosen("", &[], &[ProviderKind::Anthropic]),
            None,
            "the organization's own provider has no key"
        );
    }

    #[test]
    fn the_no_provider_message_points_to_the_settings() {
        assert!(NO_PROVIDER_MESSAGE.contains("Paramètres > Fournisseurs"));
    }
}
