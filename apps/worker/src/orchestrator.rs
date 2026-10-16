use std::{sync::Arc, time::Duration};

use capitainerie_bus_iggy::IggyEventBus;
use capitainerie_common::{CoreError, RunId};
use capitainerie_core::{
    MeetingPipeline,
    domain::{
        agent::ports::AgentRuntime,
        delegation::ports::TokenExchanger,
        similarity::ports::{Embedder, VectorStore},
    },
};
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
pub struct Orchestrator<R, X, E, V> {
    pipeline: Arc<MeetingPipeline<R, X, E, V>>,
    bus: IggyEventBus,
    slots: Arc<Semaphore>,
}

impl<R, X, E, V> Orchestrator<R, X, E, V>
where
    R: AgentRuntime + 'static,
    X: TokenExchanger + 'static,
    E: Embedder + 'static,
    V: VectorStore + 'static,
{
    pub fn new(
        pipeline: Arc<MeetingPipeline<R, X, E, V>>,
        bus: IggyEventBus,
        concurrency: usize,
    ) -> Self {
        Self {
            pipeline,
            bus,
            slots: Arc::new(Semaphore::new(concurrency.max(1))),
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
        let pipeline = Arc::clone(&self.pipeline);
        let slots = Arc::clone(&self.slots);
        tasks.spawn(async move {
            let Ok(_slot) = slots.acquire_owned().await else {
                return;
            };
            if let Err(error) = pipeline.execute(run_id, traceparent.as_deref()).await {
                tracing::error!(%run_id, %error, "run execution failed");
            }
        });
    }
}
