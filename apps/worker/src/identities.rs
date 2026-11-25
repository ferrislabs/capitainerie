use std::time::Duration;

use capitainerie_auth::FerrisKeyProvisioner;
use capitainerie_core::CapitainerieUseCase;
use chrono::Utc;
use tokio::sync::watch;

const TICK_EVERY: Duration = Duration::from_secs(10);
/// How long a refused provisioning waits before the next attempt.
const RETRY_AFTER: Duration = Duration::from_secs(60);
const BATCH: i64 = 20;

/// Keeps the agents' FerrisKey clients in line with Capitainerie: creates
/// the client of a new agent, retries the ones FerrisKey refused, and
/// disables or enables a client when its agent is archived or comes back.
/// Every step converges, so several workers may run it side by side.
pub struct IdentitySweeper {
    usecase: CapitainerieUseCase,
    provisioner: FerrisKeyProvisioner,
}

impl IdentitySweeper {
    pub fn new(usecase: CapitainerieUseCase, provisioner: FerrisKeyProvisioner) -> Self {
        Self {
            usecase,
            provisioner,
        }
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

    /// Sweeps until a sweep leaves nothing for the next one.
    async fn drain(&self) {
        let retry = chrono::Duration::from_std(RETRY_AFTER).unwrap_or(chrono::Duration::minutes(1));
        loop {
            let swept = self
                .usecase
                .sweep_agent_identities(&self.provisioner, Utc::now() - retry, BATCH)
                .await;
            match swept {
                Ok(sweep) => {
                    if sweep.picked > 0 || sweep.adopted > 0 {
                        tracing::info!(
                            adopted = sweep.adopted,
                            activated = sweep.activated,
                            synced = sweep.synced,
                            failed = sweep.failed,
                            "agent identities swept"
                        );
                    }
                    if i64::try_from(sweep.picked).unwrap_or(i64::MAX) < BATCH
                        || sweep.failed == sweep.picked
                    {
                        return;
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "the agent identity sweep failed");
                    return;
                }
            }
        }
    }
}
