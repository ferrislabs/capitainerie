mod card;
mod organization;
mod pipeline;
mod run;

use std::sync::Arc;

use capitainerie_common::CoreError;
use sqlx::PgPool;

use crate::infrastructure::{
    authorization::{AuditedAuthorizer, LocalPolicyEngine},
    crypto::SecretCipher,
    postgres,
};

pub use pipeline::{MeetingPipeline, PipelineConfig, RunContext};
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
}

impl CapitainerieUseCase {
    pub fn new(pool: PgPool, authz: CapitainerieAuthorizer) -> Self {
        Self {
            pool,
            authz,
            cipher: None,
        }
    }

    pub fn with_cipher(mut self, cipher: SecretCipher) -> Self {
        self.cipher = Some(Arc::new(cipher));
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
