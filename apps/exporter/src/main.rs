mod batcher;

use std::time::{Duration, Instant};

use anyhow::Context;
use capitainerie_analytics::{AnalyticsArgs, Dataset, EventRecord, ParquetEventWriter};
use capitainerie_args::{LogArgs, ObservabilityArgs};
use capitainerie_bus_iggy::{IggyArgs, IggyEventBus};
use capitainerie_events::Topic;
use clap::Parser;
use tokio::sync::watch;

use crate::batcher::Batcher;

const CONSUMER_GROUP: &str = "analytics-exporter";
const POLL_BATCH: u32 = 200;
const IDLE: Duration = Duration::from_secs(1);
const RETRY: Duration = Duration::from_secs(2);

#[derive(Debug, Parser)]
#[command(name = "capitainerie-exporter", version)]
struct Args {
    #[command(flatten)]
    iggy: IggyArgs,
    #[command(flatten)]
    analytics: AnalyticsArgs,
    #[arg(
        long = "analytics-flush-events",
        env = "ANALYTICS_FLUSH_EVENTS",
        default_value_t = 500
    )]
    flush_events: usize,
    #[arg(
        long = "analytics-flush-secs",
        env = "ANALYTICS_FLUSH_SECS",
        default_value_t = 30
    )]
    flush_secs: u64,
    #[command(flatten)]
    log: LogArgs,
    #[command(flatten)]
    observability: ObservabilityArgs,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    let _telemetry =
        capitainerie_telemetry::init("capitainerie-exporter", &args.log, &args.observability)?;

    let writer = ParquetEventWriter::new(Dataset::parse(&args.analytics.uri)?);
    let bus = IggyEventBus::connect(&args.iggy)
        .await
        .context("connecting to iggy")?;
    let mut batcher = Batcher::new(args.flush_events, Duration::from_secs(args.flush_secs));

    let (stop, mut stopped) = watch::channel(false);
    tokio::spawn(async move {
        capitainerie_telemetry::shutdown_signal().await;
        let _ = stop.send(true);
    });
    tracing::info!(
        uri = %args.analytics.uri,
        flush_events = args.flush_events,
        flush_secs = args.flush_secs,
        "exporter started"
    );

    while !*stopped.borrow() {
        let idle = match bus.poll(Topic::Runs, CONSUMER_GROUP, POLL_BATCH).await {
            Ok(envelopes) => {
                let now = Instant::now();
                let idle = envelopes.is_empty();
                for envelope in &envelopes {
                    match EventRecord::from_envelope(envelope) {
                        Ok(Some(record)) => batcher.push(record, now),
                        Ok(None) => {}
                        Err(error) => {
                            tracing::warn!(%error, event_id = %envelope.id, kind = %envelope.kind, "skipping malformed event");
                        }
                    }
                }
                idle
            }
            Err(error) => {
                tracing::warn!(%error, "polling failed, retrying");
                tokio::select! {
                    _ = tokio::time::sleep(RETRY) => {}
                    _ = stopped.changed() => {}
                }
                false
            }
        };
        if batcher.is_due(Instant::now()) {
            flush(&mut batcher, &writer).await;
        }
        if idle {
            tokio::select! {
                _ = tokio::time::sleep(IDLE) => {}
                _ = stopped.changed() => {}
            }
        }
    }

    tracing::info!(buffered = batcher.len(), "exporter stopping");
    flush(&mut batcher, &writer).await;
    Ok(())
}

async fn flush(batcher: &mut Batcher, writer: &ParquetEventWriter) {
    if batcher.is_empty() {
        return;
    }
    let records = batcher.take();
    let writer = writer.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let result = writer.write(&records);
        (records, result)
    })
    .await;
    match outcome {
        Ok((records, Ok(files))) => {
            tracing::info!(
                events = records.len(),
                files = files.len(),
                "flushed to parquet"
            );
            for file in &files {
                tracing::debug!(file = %file.display(), "parquet file written");
            }
        }
        Ok((records, Err(error))) => {
            tracing::error!(%error, events = records.len(), "flush failed, keeping the events for the next attempt");
            batcher.restore(records, Instant::now());
        }
        Err(error) => tracing::error!(%error, "flush task failed"),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Args;

    #[test]
    fn the_command_line_definition_is_valid() {
        Args::command().debug_assert();
    }
}
