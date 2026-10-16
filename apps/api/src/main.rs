mod openapi;
mod router;

use anyhow::Context;
use capitainerie_args::{AuthArgs, DatabaseArgs, LogArgs, ObservabilityArgs, ServerArgs};
use capitainerie_auth::{AcceptedClients, FerrisKeyVerifier};
use capitainerie_core::{
    CapitainerieUseCase,
    infrastructure::{authorization::LocalPolicyEngine, postgres},
};
use capitainerie_handlers::AppState;
use clap::Parser;
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(name = "capitainerie-api", version)]
struct Args {
    #[command(flatten)]
    server: ServerArgs,
    #[command(flatten)]
    database: DatabaseArgs,
    #[command(flatten)]
    auth: AuthArgs,
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
        capitainerie_telemetry::init("capitainerie-api", &args.log, &args.observability)?;

    let pool = postgres::connect(&args.database.url, args.database.max_connections)
        .await
        .context("connecting to postgres")?;
    let state = AppState {
        usecase: CapitainerieUseCase::new(pool, LocalPolicyEngine),
        verifier: FerrisKeyVerifier::new(AcceptedClients::new(args.auth.accepted_clients)),
    };
    let app = router::router(state, &args.server.allowed_origins)?;

    let addr = format!("{}:{}", args.server.host, args.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "api listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "could not listen for the shutdown signal");
    }
}
