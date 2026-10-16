use std::sync::Arc;

use capitainerie_common::CoreError;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::{Mutex, MutexGuard};

use super::map_sqlx_error;

/// A cloneable handle over one live transaction, so several repositories share
/// it while domain services stay unaware of it. A use case runs on one task, so
/// the mutex only satisfies the borrow checker.
pub struct SharedTx<'tx> {
    inner: Arc<Mutex<&'tx mut Transaction<'static, Postgres>>>,
}

impl<'tx> SharedTx<'tx> {
    fn new(tx: &'tx mut Transaction<'static, Postgres>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(tx)),
        }
    }

    pub async fn lock(&self) -> MutexGuard<'_, &'tx mut Transaction<'static, Postgres>> {
        self.inner.lock().await
    }
}

impl Clone for SharedTx<'_> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Commits when `work` succeeds, rolls back otherwise.
pub async fn with_tx<F, T>(pool: &PgPool, work: F) -> Result<T, CoreError>
where
    F: AsyncFnOnce(SharedTx<'_>) -> Result<T, CoreError>,
{
    let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
    let outcome = work(SharedTx::new(&mut tx)).await;
    match outcome {
        Ok(value) => {
            tx.commit().await.map_err(map_sqlx_error)?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback().await.map_err(map_sqlx_error)?;
            Err(error)
        }
    }
}
