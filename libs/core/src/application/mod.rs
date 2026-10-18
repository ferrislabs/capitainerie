mod card;
mod execute_pipeline;
mod execute_stream;
mod execution;
mod organization;
mod pipeline;
mod provider;
mod run;

use std::sync::Arc;

use capitainerie_common::CoreError;
use sqlx::PgPool;

use crate::infrastructure::{
    authorization::{AuditedAuthorizer, LocalPolicyEngine},
    crypto::SecretCipher,
    postgres,
};

pub use execute_pipeline::{ExecuteConfig, ExecutePipeline, ExecutionContext, Executor};
pub use execution::StartedExecution;
pub use pipeline::{MeetingPipeline, PipelineConfig, RunContext};
pub use provider::{ModelListing, ProviderFailure};
pub use run::{AgentDelegation, RunTimeline, SubmittedTranscript};

pub type CapitainerieAuthorizer = AuditedAuthorizer<LocalPolicyEngine>;

/// Long-lived and cloned into every request: never hold request-scoped state here.
#[derive(Clone)]
pub struct CapitainerieUseCase {
    pub(crate) pool: PgPool,
    pub(crate) authz: CapitainerieAuthorizer,
    /// Seals the delegated tokens kept between submission and execution.
    /// `None` without `DELEGATION_KEY`: submissions that need a delegation
    /// are then refused rather than stored in clear.
    pub(crate) cipher: Option<Arc<SecretCipher>>,
    /// Seals the organizations' provider API keys, with a key distinct from
    /// the delegation one. `None` without `SECRETS_KEY`: no key can then be
    /// stored or read, and the rest of the API is unaffected.
    pub(crate) secrets: Option<Arc<SecretCipher>>,
}

impl CapitainerieUseCase {
    pub fn new(pool: PgPool, authz: CapitainerieAuthorizer) -> Self {
        Self {
            pool,
            authz,
            cipher: None,
            secrets: None,
        }
    }

    pub fn with_cipher(mut self, cipher: SecretCipher) -> Self {
        self.cipher = Some(Arc::new(cipher));
        self
    }

    pub fn with_secrets_cipher(mut self, cipher: SecretCipher) -> Self {
        self.secrets = Some(Arc::new(cipher));
        self
    }

    pub(crate) fn cipher(&self) -> Result<&SecretCipher, CoreError> {
        self.cipher
            .as_deref()
            .ok_or_else(|| CoreError::Internal("DELEGATION_KEY is not configured".into()))
    }

    /// The default assembly: the local policy engine, every decision audited.
    pub fn with_local_policies(pool: PgPool) -> Self {
        let authz = AuditedAuthorizer::new(LocalPolicyEngine, pool.clone());
        Self::new(pool, authz)
    }

    pub async fn readiness(&self) -> Result<(), CoreError> {
        postgres::ping(&self.pool).await
    }
}
