mod openapi;
mod router;

use anyhow::Context;
use capitainerie_args::{
    AuthArgs, DatabaseArgs, DelegationArgs, LogArgs, ObservabilityArgs, ServerArgs,
};
use capitainerie_auth::{AcceptedClients, ExchangeArgs, FerrisKeyVerifier};
use capitainerie_core::{
    CapitainerieUseCase,
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::{AppState, Delegation};
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
    exchange: ExchangeArgs,
    #[command(flatten)]
    delegation: DelegationArgs,
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
    let mut usecase = CapitainerieUseCase::with_local_policies(pool);
    let delegation = match (&args.delegation.key, &args.exchange.client_id) {
        (Some(key), Some(_)) => {
            usecase = usecase.with_cipher(SecretCipher::from_base64(key)?);
            Some(Delegation {
                exchanger: args.exchange.exchanger()?,
                agents: args.delegation.agents.clone(),
            })
        }
        _ => {
            tracing::warn!(
                "DELEGATION_KEY or EXCHANGE_CLIENT_ID unset: runs are queued without delegation and cannot write cards"
            );
            None
        }
    };
    let state = AppState {
        usecase,
        verifier: verifier(args.auth),
        delegation,
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

fn verifier(auth: AuthArgs) -> FerrisKeyVerifier {
    let accepted = AcceptedClients::new(auth.accepted_clients);
    match auth.audience {
        Some(audience) => FerrisKeyVerifier::with_audience(accepted, audience),
        None => {
            tracing::warn!("AUTH_AUDIENCE is unset: tokens are accepted whatever their audience");
            FerrisKeyVerifier::new(accepted)
        }
    }
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "could not listen for the shutdown signal");
    }
}
