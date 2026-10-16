mod card;
mod organization;
mod run;

use capitainerie_common::CoreError;
use sqlx::PgPool;

use crate::infrastructure::{
    authorization::{AuditedAuthorizer, LocalPolicyEngine},
    postgres,
};

pub use run::SubmittedTranscript;

pub type CapitainerieAuthorizer = AuditedAuthorizer<LocalPolicyEngine>;

/// Long-lived and cloned into every request: never hold request-scoped state here.
#[derive(Clone)]
pub struct CapitainerieUseCase {
    pub(crate) pool: PgPool,
    pub(crate) authz: CapitainerieAuthorizer,
}

impl CapitainerieUseCase {
    pub fn new(pool: PgPool, authz: CapitainerieAuthorizer) -> Self {
        Self { pool, authz }
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
