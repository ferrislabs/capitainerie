use std::future::Future;

use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        execution::{ExecutionMode, RepositoryRef, TaskGrant},
    },
};
use capitainerie_github::GithubApps;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct CredentialsError(pub String);

impl CredentialsError {
    fn from_display(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

pub trait Credentials: Send + Sync + 'static {
    fn grant(
        &self,
        token_sha256: [u8; 32],
    ) -> impl Future<Output = Result<Option<TaskGrant>, CredentialsError>> + Send;

    /// The Anthropic key of the run: its initiator's own one first, then the
    /// organization's.
    fn anthropic_key(
        &self,
        organization_id: OrganizationId,
        initiated_by: Option<&str>,
    ) -> impl Future<Output = Result<Option<SecretToken>, CredentialsError>> + Send;

    /// An installation token over the run's repository; a read-only run gets
    /// one that can only read its contents.
    fn github_token(
        &self,
        organization_id: OrganizationId,
        repository: &RepositoryRef,
        mode: ExecutionMode,
    ) -> impl Future<Output = Result<Option<SecretToken>, CredentialsError>> + Send;
}

pub struct Live {
    usecase: CapitainerieUseCase,
    github: GithubApps,
}

impl Live {
    pub fn new(usecase: CapitainerieUseCase, github: GithubApps) -> Self {
        Self { usecase, github }
    }
}

impl Credentials for Live {
    async fn grant(&self, token_sha256: [u8; 32]) -> Result<Option<TaskGrant>, CredentialsError> {
        self.usecase
            .find_task_grant(token_sha256)
            .await
            .map_err(CredentialsError::from_display)
    }

    async fn anthropic_key(
        &self,
        organization_id: OrganizationId,
        initiated_by: Option<&str>,
    ) -> Result<Option<SecretToken>, CredentialsError> {
        Ok(self
            .usecase
            .executor_anthropic_access(organization_id, initiated_by)
            .await
            .map_err(CredentialsError::from_display)?
            .map(|access| access.secret))
    }

    async fn github_token(
        &self,
        organization_id: OrganizationId,
        repository: &RepositoryRef,
        mode: ExecutionMode,
    ) -> Result<Option<SecretToken>, CredentialsError> {
        let Some(github) = self
            .github
            .client(organization_id)
            .await
            .map_err(CredentialsError::from_display)?
        else {
            return Ok(None);
        };
        let Some(installation) = self
            .usecase
            .github_installation_for(organization_id)
            .await
            .map_err(CredentialsError::from_display)?
        else {
            return Ok(None);
        };
        let token = match mode {
            ExecutionMode::Change => {
                github
                    .installation_token(installation.installation_id, repository)
                    .await
            }
            ExecutionMode::Audit => {
                github
                    .read_only_installation_token(installation.installation_id, repository)
                    .await
            }
        }
        .map_err(CredentialsError::from_display)?;
        Ok(Some(token.token))
    }
}
