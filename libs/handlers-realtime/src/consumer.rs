use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use capitainerie_bus_iggy::{BusError, IggyArgs, IggyEventBus, Subscription};
use capitainerie_events::Topic;
use tokio::task::JoinHandle;

use crate::{hub::Hub, wire::Notice};

const GROUP: &str = "api-realtime";
const BATCH: u32 = 100;
const IDLE: Duration = Duration::from_millis(250);
const FIRST_RETRY: Duration = Duration::from_secs(1);
const LAST_RETRY: Duration = Duration::from_secs(30);

/// Feeds the hub from Iggy for the life of the process. Never returns and never
/// fails: while Iggy is out of reach the hub is degraded and the connection
/// is retried with a growing delay.
pub fn spawn(args: IggyArgs, hub: Hub) -> JoinHandle<()> {
    spawn_until(args, hub, std::future::pending())
}

/// Like [`spawn`], but once `shutdown` completes the task closes its
/// ephemeral consumer groups and ends. Await the returned handle after the
/// shutdown signal so the groups are deleted before the process exits.
pub fn spawn_until<S>(args: IggyArgs, hub: Hub, shutdown: S) -> JoinHandle<()>
where
    S: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move { run(args, hub, shutdown).await })
}

async fn run<S: Future<Output = ()>>(args: IggyArgs, hub: Hub, shutdown: S) {
    tokio::pin!(shutdown);
    let mut retry = FIRST_RETRY;
    loop {
        if session(&args, &hub, &mut retry, &mut shutdown).await == Outcome::Stopped {
            return;
        }
        hub.set_degraded(true);
        tokio::select! {
            () = &mut shutdown => return,
            () = tokio::time::sleep(retry) => {}
        }
        retry = (retry * 2).min(LAST_RETRY);
    }
}

#[derive(PartialEq, Eq)]
enum Outcome {
    Lost,
    Stopped,
}

async fn session<S: Future<Output = ()>>(
    args: &IggyArgs,
    hub: &Hub,
    retry: &mut Duration,
    shutdown: &mut Pin<&mut S>,
) -> Outcome {
    let connected = tokio::select! {
        () = &mut *shutdown => return Outcome::Stopped,
        connected = IggyEventBus::connect(args) => connected,
    };
    let bus = match connected {
        Ok(bus) => bus,
        Err(error) => {
            tracing::warn!(%error, "iggy unreachable: realtime is degraded, retrying");
            return Outcome::Lost;
        }
    };
    let mut subscriptions = Vec::new();
    for topic in Topic::ALL {
        match bus.subscribe_ephemeral(topic, GROUP).await {
            Ok(subscription) => subscriptions.push((topic, subscription)),
            Err(error) => {
                tracing::warn!(%error, "realtime consumer could not subscribe");
                close_all(subscriptions).await;
                return Outcome::Lost;
            }
        }
    }
    *retry = FIRST_RETRY;
    tracing::info!("realtime consumer connected");
    hub.set_degraded(false);
    let stopped = tokio::select! {
        () = &mut *shutdown => true,
        result = pump(&mut subscriptions, hub) => {
            if let Err(error) = result {
                tracing::warn!(%error, "realtime consumer lost the bus");
            }
            false
        }
    };
    close_all(subscriptions).await;
    if stopped {
        Outcome::Stopped
    } else {
        Outcome::Lost
    }
}

async fn close_all(subscriptions: Vec<(Topic, Subscription)>) {
    for (topic, subscription) in subscriptions {
        if let Err(error) = subscription.close().await {
            tracing::warn!(%error, topic = topic.as_str(), "could not close a realtime subscription");
        }
    }
}

async fn pump(subscriptions: &mut [(Topic, Subscription)], hub: &Hub) -> Result<(), BusError> {
    loop {
        let mut seen = 0;
        for (_, subscription) in subscriptions.iter_mut() {
            let envelopes = subscription.next_batch(BATCH).await?;
            seen += envelopes.len();
            for envelope in &envelopes {
                hub.publish(Notice::from_envelope(envelope));
            }
        }
        if seen == 0 {
            tokio::time::sleep(IDLE).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_ends_the_task_while_the_bus_is_unreachable() {
        let args = IggyArgs {
            address: "127.0.0.1:1".into(),
            username: "iggy".into(),
            password: "iggy".into(),
            stream: "unreachable".into(),
            partitions: 1,
        };
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let handle = spawn_until(args, Hub::new(16), async {
            let _ = stopped.await;
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .unwrap()
            .unwrap();
    }
}
