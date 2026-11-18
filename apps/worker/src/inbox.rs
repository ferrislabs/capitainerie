use std::time::Duration;

use capitainerie_core::CapitainerieUseCase;
use tokio::sync::watch;

const TICK_EVERY: Duration = Duration::from_secs(2);
const BATCH: i64 = 200;
/// How old an outbox event must be before the projection reads it, so a
/// transaction committing a little after minting its event ids is not skipped.
const SETTLE_SECS: i64 = 5;

/// Keeps the members' inboxes in step with the outbox. The projection's
/// cursor is locked per batch, so several workers can run it side by side.
pub struct InboxProjector {
    usecase: CapitainerieUseCase,
}

impl InboxProjector {
    pub fn new(usecase: CapitainerieUseCase) -> Self {
        Self { usecase }
    }

    pub async fn run(self, mut stopped: watch::Receiver<bool>) {
        let mut every = tokio::time::interval(TICK_EVERY);
        every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = stopped.wait_for(|stop| *stop) => break,
                _ = every.tick() => self.drain().await,
            }
        }
    }

    async fn drain(&self) {
        loop {
            match self.usecase.project_inbox(BATCH, SETTLE_SECS).await {
                Ok(read) if i64::try_from(read).unwrap_or(i64::MAX) < BATCH => return,
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%error, "the inbox projection failed");
                    return;
                }
            }
        }
    }
}
