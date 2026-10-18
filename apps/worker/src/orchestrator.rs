use std::{
    collections::HashSet,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use crate::dispatch::RunPipelines;
use capitainerie_bus_iggy::IggyEventBus;
use capitainerie_common::{CoreError, RunId};
use capitainerie_events::{Topic, run::RunQueued};
use tokio::{
    sync::{Semaphore, watch},
    task::JoinSet,
};

const CONSUMER_GROUP: &str = "orchestrator";
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

/// Iggy wakes the orchestrator up; Postgres decides. A run is only executed
/// once claimed in the database, so a lost or duplicated event costs at most
/// one sweep interval, never a run.
pub struct Orchestrator<P> {
    pipeline: Arc<P>,
    bus: IggyEventBus,
    slots: Arc<Semaphore>,
    execution_slots: Arc<Semaphore>,
    in_flight: InFlight,
}

/// The runs this worker is working on: an event and a sweep naming the same
/// run must not start it twice.
#[derive(Clone, Default)]
struct InFlight(Arc<Mutex<HashSet<RunId>>>);

struct InFlightClaim {
    set: InFlight,
    run_id: RunId,
}

impl InFlight {
    fn claim(&self, run_id: RunId) -> Option<InFlightClaim> {
        let inserted = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(run_id);
        inserted.then(|| InFlightClaim {
            set: self.clone(),
            run_id,
        })
    }
}

impl Drop for InFlightClaim {
    fn drop(&mut self) {
        self.set
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.run_id);
    }
}

impl<P: RunPipelines> Orchestrator<P> {
    pub fn new(
        pipeline: Arc<P>,
        bus: IggyEventBus,
        concurrency: usize,
        execution_concurrency: usize,
    ) -> Self {
        Self {
            pipeline,
            bus,
            slots: Arc::new(Semaphore::new(concurrency.max(1))),
            execution_slots: Arc::new(Semaphore::new(execution_concurrency.max(1))),
            in_flight: InFlight::default(),
        }
    }

    pub async fn run(self, mut stopped: watch::Receiver<bool>) -> Result<(), CoreError> {
        let mut tasks = JoinSet::new();
        self.sweep(&mut tasks).await;
        let mut sweep = tokio::time::interval(SWEEP_INTERVAL);
        let mut poll = tokio::time::interval(POLL_INTERVAL);
        loop {
            tokio::select! {
                _ = stopped.wait_for(|stop| *stop) => break,
                _ = sweep.tick() => self.sweep(&mut tasks).await,
                _ = poll.tick() => self.consume(&mut tasks).await,
                Some(joined) = tasks.join_next(), if !tasks.is_empty() => {
                    if let Err(error) = joined {
                        tracing::error!(%error, "a run task panicked");
                    }
                }
            }
        }
        tracing::info!(running = tasks.len(), "waiting for running runs to stop");
        tasks.shutdown().await;
        Ok(())
    }

    async fn consume(&self, tasks: &mut JoinSet<()>) {
        let envelopes = match self.bus.poll(Topic::Runs, CONSUMER_GROUP, 10).await {
            Ok(envelopes) => envelopes,
            Err(error) => {
                tracing::warn!(%error, "polling runs failed");
                return;
            }
        };
        for envelope in envelopes {
            match envelope.open::<RunQueued>() {
                Some(Ok(queued)) => {
                    self.spawn(tasks, RunId(queued.run_id), envelope.traceparent.clone())
                }
                Some(Err(error)) => {
                    tracing::warn!(%error, event_id = %envelope.id, "undecodable run.queued")
                }
                None => {}
            }
        }
    }

    /// Picks up queued runs whose event was missed and runs whose worker died.
    async fn sweep(&self, tasks: &mut JoinSet<()>) {
        let stale_before = self.pipeline.stale_before();
        match self.pipeline.usecase().pending_runs(stale_before, 20).await {
            Ok(runs) => {
                for run_id in runs {
                    self.spawn(tasks, run_id, None);
                }
            }
            Err(error) => tracing::warn!(%error, "sweeping pending runs failed"),
        }
    }

    fn spawn(&self, tasks: &mut JoinSet<()>, run_id: RunId, traceparent: Option<String>) {
        let Some(claim) = self.in_flight.claim(run_id) else {
            tracing::debug!(%run_id, "run already in flight on this worker");
            return;
        };
        let pipeline = Arc::clone(&self.pipeline);
        let meeting_slots = Arc::clone(&self.slots);
        let execution_slots = Arc::clone(&self.execution_slots);
        tasks.spawn(async move {
            let _claim = claim;
            let slots = match pipeline.is_execution(run_id).await {
                Ok(true) => execution_slots,
                Ok(false) => meeting_slots,
                Err(error) => {
                    tracing::warn!(%run_id, %error, "cannot tell what kind of run this is");
                    return;
                }
            };
            let Ok(_slot) = slots.acquire_owned().await else {
                return;
            };
            if let Err(error) = pipeline.execute(run_id, traceparent.as_deref()).await {
                tracing::error!(%run_id, %error, "run execution failed");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_in_flight_cannot_be_claimed_twice_until_it_ends() {
        let in_flight = InFlight::default();
        let run = RunId::new();

        let first = in_flight.claim(run);
        assert!(first.is_some());
        assert!(in_flight.claim(run).is_none());
        assert!(in_flight.claim(RunId::new()).is_some());

        drop(first);
        assert!(in_flight.claim(run).is_some());
    }
}
