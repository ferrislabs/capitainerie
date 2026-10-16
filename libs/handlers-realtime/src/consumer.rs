use std::time::Duration;

use capitainerie_bus_iggy::{BusError, IggyArgs, IggyEventBus};
use capitainerie_events::{EventEnvelope, Topic};
use chrono::{DateTime, Utc};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{hub::Hub, wire::Notice};

const BATCH: u32 = 100;
const IDLE: Duration = Duration::from_millis(250);
const FIRST_RETRY: Duration = Duration::from_secs(1);
const LAST_RETRY: Duration = Duration::from_secs(30);

/// A fresh consumer group replays the topic from its start, so what predates
/// the process is skipped: sockets only ever see events from now on.
fn fresh(envelope: &EventEnvelope, since: DateTime<Utc>) -> bool {
    envelope.occurred_at >= since
}

/// Feeds the hub from Iggy for the life of the process. Never returns and never
/// fails: while Iggy is out of reach the hub is degraded and the connection
/// is retried with a growing delay.
pub fn spawn(args: IggyArgs, hub: Hub) -> JoinHandle<()> {
    tokio::spawn(async move { run(args, hub).await })
}

async fn run(args: IggyArgs, hub: Hub) {
    let since = Utc::now();
    let group = format!("api-realtime-{}", Uuid::now_v7());
    let mut retry = FIRST_RETRY;
    loop {
        match IggyEventBus::connect(&args).await {
            Ok(bus) => {
                retry = FIRST_RETRY;
                tracing::info!(%group, "realtime consumer connected");
                hub.set_degraded(false);
                if let Err(error) = pump(&bus, &hub, &group, since).await {
                    tracing::warn!(%error, "realtime consumer lost the bus");
                }
            }
            Err(error) => {
                tracing::warn!(%error, "iggy unreachable: realtime is degraded, retrying");
            }
        }
        hub.set_degraded(true);
        tokio::time::sleep(retry).await;
        retry = (retry * 2).min(LAST_RETRY);
    }
}

async fn pump(
    bus: &IggyEventBus,
    hub: &Hub,
    group: &str,
    since: DateTime<Utc>,
) -> Result<(), BusError> {
    loop {
        let mut seen = 0;
        for topic in Topic::ALL {
            match bus.poll(topic, group, BATCH).await {
                Ok(envelopes) => {
                    seen += envelopes.len();
                    for envelope in envelopes.iter().filter(|e| fresh(e, since)) {
                        hub.publish(Notice::from_envelope(envelope));
                    }
                }
                Err(BusError::Decode(error)) => {
                    tracing::warn!(%error, topic = topic.as_str(), "skipping an undecodable event");
                }
                Err(error) => return Err(error),
            }
        }
        if seen == 0 {
            tokio::time::sleep(IDLE).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration as Span;

    use super::*;

    fn at(occurred_at: DateTime<Utc>) -> EventEnvelope {
        EventEnvelope {
            id: Uuid::now_v7(),
            kind: "run.queued".into(),
            topic: Topic::Runs,
            organization_id: Uuid::now_v7(),
            partition_key: Uuid::now_v7(),
            occurred_at,
            traceparent: None,
            payload: serde_json::Value::Null,
        }
    }

    #[test]
    fn history_older_than_the_process_is_skipped() {
        let since = Utc::now();
        assert!(!fresh(&at(since - Span::seconds(1)), since));
        assert!(fresh(&at(since), since));
        assert!(fresh(&at(since + Span::seconds(1)), since));
    }
}
