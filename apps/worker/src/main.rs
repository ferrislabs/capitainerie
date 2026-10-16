use anyhow::Context;
use capitainerie_args::{DatabaseArgs, LogArgs, ObservabilityArgs};
use capitainerie_bus_iggy::{IggyArgs, IggyEventBus};
use capitainerie_core::infrastructure::{
    event::{OutboxRelay, OutboxRelayConfig},
    postgres,
};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "capitainerie-worker", version)]
struct Args {
    #[command(flatten)]
    database: DatabaseArgs,
    #[command(flatten)]
    iggy: IggyArgs,
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
        capitainerie_telemetry::init("capitainerie-worker", &args.log, &args.observability)?;

    let pool = postgres::connect(&args.database.url, args.database.max_connections)
        .await
        .context("connecting to postgres")?;
    let bus = IggyEventBus::connect(&args.iggy)
        .await
        .context("connecting to iggy")?;

    tracing::info!(stream = %args.iggy.stream, "worker started, relaying the outbox");
    OutboxRelay::new(pool, bus, OutboxRelayConfig::default())
        .run(shutdown_signal())
        .await?;
    tracing::info!("worker stopped");
    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "could not listen for the shutdown signal");
    }
}
