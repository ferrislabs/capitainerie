use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
    time::Duration,
};

use aws_lc_rs::digest::{SHA256, digest};
use capitainerie_common::CoreError;
use capitainerie_core::domain::provider::{ModelTarget, ProviderAccess, ProviderKind};
use rig_core::{
    DynModel,
    operation::Completion,
    providers::openai::{OpenAI, OpenAIConfig, wire::MISTRAL},
};
use rig_reqwest::{ReqwestClient, reqwest};

use crate::{anthropic::Api, args::LlmArgs, error::RuntimeError};

pub(crate) const ANTHROPIC_BASE: &str = "https://api.anthropic.com";
pub(crate) const MISTRAL_BASE: &str = "https://api.mistral.ai";
const UNUSED_KEY: &str = "unused";

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClientKey {
    kind: ProviderKind,
    base_url: String,
    secret_sha256: [u8; 32],
}

impl std::fmt::Debug for ClientKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientKey")
            .field("kind", &self.kind)
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl ClientKey {
    fn of(access: &ProviderAccess) -> Self {
        let mut secret_sha256 = [0u8; 32];
        secret_sha256.copy_from_slice(digest(&SHA256, access.secret.expose().as_bytes()).as_ref());
        Self {
            kind: access.kind,
            base_url: access.base_url.clone().unwrap_or_default(),
            secret_sha256,
        }
    }
}

#[derive(Clone)]
pub(crate) enum Client {
    Chat(Box<OpenAI>),
    Anthropic(Api),
}

pub(crate) enum Engine {
    Chat(DynModel<Completion>),
    Anthropic(Api),
}

pub(crate) struct Resolved {
    pub client: Client,
    pub provider: &'static str,
    pub model: String,
}

impl Resolved {
    pub(crate) fn engine(&self) -> Engine {
        match &self.client {
            Client::Chat(client) => Engine::Chat(client.chat(self.model.as_str()).into()),
            Client::Anthropic(api) => Engine::Anthropic(api.clone()),
        }
    }
}

pub(crate) struct Clients {
    http: reqwest::Client,
    environment: OpenAI,
    cache: Mutex<HashMap<ClientKey, Client>>,
}

impl Clients {
    pub(crate) fn new(args: &LlmArgs) -> Result<Self, RuntimeError> {
        let http = http_client(args.timeout_secs)?;
        let environment = OpenAIConfig::new(args.api_key.as_str())
            .with_base_url(args.base_url.as_str())
            .connect(ReqwestClient::from(http.clone()));
        Ok(Self {
            http,
            environment,
            cache: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn resolve(&self, target: &ModelTarget) -> Result<Resolved, CoreError> {
        let model = target.model.model.clone();
        let Some(access) = &target.access else {
            return match target.model.provider {
                ProviderKind::OpenAiCompatible => Ok(Resolved {
                    client: Client::Chat(Box::new(self.environment.clone())),
                    provider: "openai",
                    model,
                }),
                kind => Err(CoreError::Invalid(format!(
                    "no {} key configured for this organization",
                    kind.as_str()
                ))),
            };
        };
        let key = ClientKey::of(access);
        let cached = self
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
            .cloned();
        let client = match cached {
            Some(client) => client,
            None => {
                let built = self.build(access)?;
                self.cache
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(key, built.clone());
                built
            }
        };
        Ok(Resolved {
            client,
            provider: provider_name(access.kind),
            model,
        })
    }

    #[cfg(test)]
    pub(crate) fn cached(&self) -> usize {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    fn build(&self, access: &ProviderAccess) -> Result<Client, CoreError> {
        let base_url = access
            .base_url
            .as_deref()
            .map(|url| url.trim_end_matches('/'));
        let key = match access.secret.expose() {
            "" => UNUSED_KEY,
            key => key,
        };
        match access.kind {
            ProviderKind::OpenAiCompatible => {
                let base_url = base_url.ok_or_else(|| {
                    CoreError::Invalid(
                        "the OpenAI-compatible provider has no base url configured".into(),
                    )
                })?;
                Ok(Client::Chat(Box::new(
                    OpenAIConfig::new(key)
                        .with_base_url(base_url)
                        .connect(ReqwestClient::from(self.http.clone())),
                )))
            }
            ProviderKind::Mistral => Ok(Client::Chat(Box::new(
                OpenAIConfig::with_key(&MISTRAL, key)
                    .with_base_url(base_url.unwrap_or(MISTRAL_BASE))
                    .connect(ReqwestClient::from(self.http.clone())),
            ))),
            ProviderKind::Anthropic => Ok(Client::Anthropic(Api::new(
                self.http.clone(),
                base_url.unwrap_or(ANTHROPIC_BASE),
                access.secret.clone(),
            ))),
        }
    }
}

pub(crate) fn provider_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::Mistral => "mistral",
        ProviderKind::OpenAiCompatible => "openai",
    }
}

pub(crate) fn http_client(timeout_secs: u64) -> Result<reqwest::Client, RuntimeError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|error| RuntimeError::HttpClient(error.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use capitainerie_core::domain::{agent::SecretToken, provider::ModelRef};

    use super::*;

    fn clients() -> Clients {
        Clients::new(&LlmArgs {
            base_url: "http://127.0.0.1:1/v1".into(),
            api_key: "env-key".into(),
            model: "m".into(),
            timeout_secs: 5,
            mcp_url: "http://127.0.0.1:1/mcp".into(),
        })
        .unwrap()
    }

    fn target(kind: ProviderKind, secret: &str, base_url: Option<&str>) -> ModelTarget {
        ModelTarget {
            model: ModelRef {
                provider: kind,
                model: "m".into(),
            },
            access: Some(ProviderAccess {
                kind,
                secret: SecretToken::new(secret),
                base_url: base_url.map(str::to_owned),
            }),
        }
    }

    #[test]
    fn a_client_is_built_once_per_provider_url_and_key() {
        let clients = clients();
        let anthropic = target(ProviderKind::Anthropic, "sk-ant-one", None);
        clients.resolve(&anthropic).unwrap();
        clients.resolve(&anthropic).unwrap();
        assert_eq!(clients.cached(), 1);

        clients
            .resolve(&target(ProviderKind::Anthropic, "sk-ant-two", None))
            .unwrap();
        clients
            .resolve(&target(ProviderKind::Mistral, "sk-ant-one", None))
            .unwrap();
        clients
            .resolve(&target(
                ProviderKind::OpenAiCompatible,
                "sk-ant-one",
                Some("http://a/v1"),
            ))
            .unwrap();
        clients
            .resolve(&target(
                ProviderKind::OpenAiCompatible,
                "sk-ant-one",
                Some("http://b/v1"),
            ))
            .unwrap();
        assert_eq!(clients.cached(), 5);
    }

    #[test]
    fn the_cache_key_never_shows_the_secret() {
        let access = ProviderAccess {
            kind: ProviderKind::Anthropic,
            secret: SecretToken::new("sk-ant-very-secret"),
            base_url: None,
        };
        let key = ClientKey::of(&access);
        assert!(!format!("{key:?}").contains("very-secret"));
        assert!(!key.secret_sha256.is_empty());
        let other = ClientKey::of(&ProviderAccess {
            secret: SecretToken::new("sk-ant-other"),
            ..access
        });
        assert_ne!(key, other);
    }

    #[test]
    fn without_credentials_only_the_openai_compatible_provider_uses_the_worker_endpoint() {
        let clients = clients();
        let local = ModelTarget {
            model: ModelRef {
                provider: ProviderKind::OpenAiCompatible,
                model: "qwen".into(),
            },
            access: None,
        };
        let resolved = clients.resolve(&local).unwrap();
        assert_eq!(resolved.provider, "openai");
        assert_eq!(clients.cached(), 0);
        let anthropic = ModelTarget {
            model: ModelRef {
                provider: ProviderKind::Anthropic,
                model: "claude-haiku-5-5".into(),
            },
            access: None,
        };
        assert!(matches!(
            clients.resolve(&anthropic),
            Err(CoreError::Invalid(message)) if message.contains("no anthropic key")
        ));
    }

    #[test]
    fn an_openai_compatible_key_needs_a_base_url() {
        let error = clients()
            .resolve(&target(ProviderKind::OpenAiCompatible, "", None))
            .err()
            .unwrap();
        assert!(matches!(error, CoreError::Invalid(message) if message.contains("base url")));
    }
}
