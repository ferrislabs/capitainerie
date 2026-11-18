use std::time::Duration;

use capitainerie_core::CapitainerieUseCase;
use chrono::Utc;
use tokio::sync::watch;

const TICK_EVERY: Duration = Duration::from_secs(30);
const BATCH: i64 = 20;

/// Starts the routines' due passages. Each tick claims its triggers with
/// `FOR UPDATE SKIP LOCKED` and reschedules them in the same transaction,
/// so several workers can run it side by side without running a passage twice.
pub struct RoutineScheduler {
    usecase: CapitainerieUseCase,
}

impl RoutineScheduler {
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

    /// Ticks until a tick leaves work for the next one no more.
    async fn drain(&self) {
        loop {
            match self.usecase.tick_routines(Utc::now(), BATCH).await {
                Ok(runs) => {
                    if !runs.is_empty() {
                        tracing::info!(passages = runs.len(), "routine passages started");
                    }
                    if i64::try_from(runs.len()).unwrap_or(i64::MAX) < BATCH {
                        return;
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "the routine scheduler tick failed");
                    return;
                }
            }
        }
    }
}
