use capitainerie_args::{LogArgs, ObservabilityArgs};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "capitainerie-worker", version)]
struct Args {
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

    tracing::info!("worker started, waiting for runs");
    tokio::signal::ctrl_c().await?;
    tracing::info!("worker stopping");
    Ok(())
}
