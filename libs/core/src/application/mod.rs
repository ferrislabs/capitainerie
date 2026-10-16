use capitainerie_common::CoreError;
use sqlx::PgPool;

use crate::infrastructure::postgres;

/// Long-lived and cloned into every request: never hold request-scoped state here.
#[derive(Clone)]
pub struct CapitainerieUseCase {
    pub(crate) pool: PgPool,
}

impl CapitainerieUseCase {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn readiness(&self) -> Result<(), CoreError> {
        postgres::ping(&self.pool).await
    }
}
