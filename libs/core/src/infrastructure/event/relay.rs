use std::time::Duration;

use capitainerie_common::CoreError;
use sqlx::{PgPool, postgres::PgListener};

use super::postgres::{OUTBOX_CHANNEL, claim_pending, mark_published};
use crate::{domain::event::ports::EventBus, infrastructure::postgres::map_sqlx_error};

#[derive(Debug, Clone)]
pub struct OutboxRelayConfig {
    pub batch_size: i64,
    /// Fallback when a notification is missed: a missed `NOTIFY` costs at most
    /// this delay, never an event.
    pub poll_interval: Duration,
    pub retry_delay: Duration,
}

impl Default for OutboxRelayConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            poll_interval: Duration::from_secs(2),
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// Publishes committed outbox events to the bus, then stamps them. A crash
/// between the two republishes the batch: delivery is at least once.
pub struct OutboxRelay<B> {
    pool: PgPool,
    bus: B,
    config: OutboxRelayConfig,
}

impl<B: EventBus> OutboxRelay<B> {
    pub fn new(pool: PgPool, bus: B, config: OutboxRelayConfig) -> Self {
        Self { pool, bus, config }
    }

    pub async fn run(self, shutdown: impl Future<Output = ()>) -> Result<(), CoreError> {
        let mut listener = PgListener::connect_with(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        listener
            .listen(OUTBOX_CHANNEL)
            .await
            .map_err(map_sqlx_error)?;
        tokio::pin!(shutdown);

        loop {
            match self.publish_pending().await {
                Ok(published) if published as i64 == self.config.batch_size => continue,
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%error, "outbox relay failed, retrying");
                    tokio::select! {
                        () = &mut shutdown => return Ok(()),
                        () = tokio::time::sleep(self.config.retry_delay) => continue,
                    }
                }
            }
            tokio::select! {
                () = &mut shutdown => return Ok(()),
                notification = listener.recv() => {
                    if let Err(error) = notification {
                        tracing::warn!(%error, "outbox listener lost its connection");
                    }
                }
                () = tokio::time::sleep(self.config.poll_interval) => {}
            }
        }
    }

    /// Publishes one batch; returns how many events it published.
    #[tracing::instrument(skip_all, err)]
    pub async fn publish_pending(&self) -> Result<usize, CoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let batch = claim_pending(&mut tx, self.config.batch_size).await?;
        if batch.is_empty() {
            return Ok(0);
        }
        let (ids, envelopes): (Vec<_>, Vec<_>) = batch.into_iter().unzip();
        self.bus.publish(&envelopes).await?;
        mark_published(&mut tx, &ids).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        tracing::debug!(count = ids.len(), "outbox events published");
        Ok(ids.len())
    }
}
