use std::sync::Arc;

use capitainerie_core::{CapitainerieUseCase, application::CapitainerieAuthorizer};
use capitainerie_github::GithubClient;

use crate::{error::IntegrationError, signer::StateSigner};

/// Each part is absent when its setting is: the status endpoint then says
/// `app_configured: false` and the endpoints that need it answer 503.
#[derive(Clone, Default)]
pub struct GithubIntegration {
    pub client: Option<Arc<GithubClient>>,
    pub slug: Option<String>,
    pub signer: Option<StateSigner>,
}

impl GithubIntegration {
    pub fn app_configured(&self) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.oauth_configured())
            && self.slug.is_some()
            && self.signer.is_some()
    }

    pub(crate) fn client(&self) -> Result<&GithubClient, IntegrationError> {
        self.client
            .as_deref()
            .ok_or(IntegrationError::NotConfigured(
                "GITHUB_APP_ID, GITHUB_APP_PRIVATE_KEY, GITHUB_APP_CLIENT_ID and GITHUB_APP_CLIENT_SECRET are unset",
            ))
    }

    pub(crate) fn installer(
        &self,
    ) -> Result<(&GithubClient, &str, &StateSigner), IntegrationError> {
        let client = self.client()?;
        let slug = self
            .slug
            .as_deref()
            .ok_or(IntegrationError::NotConfigured("GITHUB_APP_SLUG is unset"))?;
        let signer = self
            .signer
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("SECRETS_KEY is unset"))?;
        Ok((client, slug, signer))
    }
}

#[derive(Clone)]
pub struct IntegrationsState {
    pub usecase: CapitainerieUseCase,
    pub authz: CapitainerieAuthorizer,
    pub github: GithubIntegration,
}
