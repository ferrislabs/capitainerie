use std::sync::Arc;

use capitainerie_github::GithubApps;

use crate::{error::IntegrationError, signer::StateSigner};

#[derive(Clone)]
pub struct GithubIntegration {
    pub apps: Arc<GithubApps>,
    pub signer: Option<StateSigner>,
    pub console_url: Option<String>,
}

impl GithubIntegration {
    pub fn new(apps: Arc<GithubApps>) -> Self {
        Self {
            apps,
            signer: None,
            console_url: None,
        }
    }

    pub(crate) fn signer(&self) -> Result<&StateSigner, IntegrationError> {
        self.signer.as_ref().ok_or(IntegrationError::NotConfigured {
            code: "secrets_key_not_configured",
            message: "SECRETS_KEY is unset",
        })
    }

    pub(crate) fn console_url(&self) -> Result<&str, IntegrationError> {
        self.console_url
            .as_deref()
            .map(|url| url.trim_end_matches('/'))
            .ok_or(IntegrationError::NotConfigured {
                code: "console_url_not_configured",
                message: "CONSOLE_URL is unset",
            })
    }
}

#[derive(Clone)]
pub struct IntegrationsState {
    pub usecase: capitainerie_core::CapitainerieUseCase,
    pub github: GithubIntegration,
}
