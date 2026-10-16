use anyhow::Context;
use axum::{Router, http::StatusCode, routing::get};
use capitainerie_args::{LogArgs, ObservabilityArgs};
use clap::Parser;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

#[derive(Debug, Parser)]
#[command(name = "capitainerie-mcp-kanban", version)]
struct Args {
    #[arg(long = "mcp-host", env = "MCP_HOST", default_value = "0.0.0.0")]
    host: String,
    #[arg(long = "mcp-port", env = "MCP_PORT", default_value_t = 4101)]
    port: u16,
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
        capitainerie_telemetry::init("capitainerie-mcp-kanban", &args.log, &args.observability)?;

    let app = Router::new()
        .route("/health/live", get(|| async { StatusCode::NO_CONTENT }))
        .layer(
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
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
