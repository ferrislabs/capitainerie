use anyhow::Context;
use capitainerie_auth::{AcceptedClients, FerrisKeyVerifier};
use capitainerie_core::{CapitainerieUseCase, infrastructure::postgres};
use capitainerie_mcp_kanban::{config::Args, router};
use clap::Parser;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    let _telemetry =
        capitainerie_telemetry::init("capitainerie-mcp-kanban", &args.log, &args.observability)?;

    let pool = postgres::connect(&args.database.url, args.database.max_connections)
        .await
        .context("connecting to postgres")?;
    let usecase = CapitainerieUseCase::with_local_policies(pool);
    let verifier =
        FerrisKeyVerifier::with_audience(AcceptedClients::new(args.accepted_agents), args.audience);
    let app = router(usecase, verifier, args.public_url).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );

    let addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "mcp-kanban listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                tracing::error!(%error, "could not listen for the shutdown signal");
            }
        })
        .await?;
    Ok(())
}
