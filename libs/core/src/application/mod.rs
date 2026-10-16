mod organization;
mod run;

use capitainerie_common::CoreError;
use sqlx::PgPool;

use crate::infrastructure::{authorization::LocalPolicyEngine, postgres};

pub use run::SubmittedTranscript;

pub type CapitainerieAuthorizer = LocalPolicyEngine;

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

    pub async fn readiness(&self) -> Result<(), CoreError> {
        postgres::ping(&self.pool).await
    }
}
