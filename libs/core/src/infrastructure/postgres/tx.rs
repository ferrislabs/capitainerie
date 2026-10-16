use std::sync::Arc;

use capitainerie_common::CoreError;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::{Mutex, MutexGuard};

use super::map_sqlx_error;
use crate::infrastructure::event::{TransactionalEventEmitter, postgres::PgOutboxRepository};

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

/// Runs `work` in a transaction, writes what it emitted to the outbox **in that
/// same transaction**, then commits. Appending here rather than at the end of
/// the caller's body means an early return can never skip it.
pub async fn with_tx_emitting<F, T>(
    pool: &PgPool,
    emitter: &TransactionalEventEmitter,
    work: F,
) -> Result<T, CoreError>
where
    F: AsyncFnOnce(SharedTx<'_>) -> Result<T, CoreError>,
{
    let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
    let outcome = {
        let shared = SharedTx::new(&mut tx);
        match work(shared.clone()).await {
            Ok(value) => {
                let events = emitter.drain()?;
                PgOutboxRepository::new(&shared)
                    .append(&events)
                    .await
                    .map(|()| value)
            }
            Err(error) => Err(error),
        }
    };
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
