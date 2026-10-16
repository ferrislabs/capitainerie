mod openapi;
mod router;

use anyhow::Context;
use capitainerie_analytics::{AnalyticsArgs, DataFusionAnalytics, Dataset};
use capitainerie_args::{
    AuthArgs, DatabaseArgs, DelegationArgs, LogArgs, ObservabilityArgs, ServerArgs,
};
use capitainerie_auth::{AcceptedClients, ExchangeArgs, FerrisKeyVerifier};
use capitainerie_bus_iggy::IggyArgs;
use capitainerie_core::{
    CapitainerieUseCase,
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::{AppState, Delegation};
use capitainerie_handlers_realtime::{Gateway, Hub, consumer};
use clap::Parser;
use tokio::net::TcpListener;

const REALTIME_BACKLOG: usize = 256;

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
    iggy: IggyArgs,
    #[command(flatten)]
    analytics: AnalyticsArgs,
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
    let (stop, stopped) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        capitainerie_telemetry::shutdown_signal().await;
        let _ = stop.send(true);
    });
    let hub = Hub::new(REALTIME_BACKLOG);
    let realtime_consumer =
        consumer::spawn_until(args.iggy, hub.clone(), stopped_signal(stopped.clone()));
    let realtime = Gateway::new(state.clone(), hub);
    let analytics = DataFusionAnalytics::new(Dataset::parse(&args.analytics.uri)?);
    let app = router::router(state, realtime, analytics, &args.server.allowed_origins)?;

    let addr = format!("{}:{}", args.server.host, args.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "api listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(stopped_signal(stopped))
        .await?;
    if let Err(error) = realtime_consumer.await {
        tracing::error!(%error, "the realtime consumer did not stop cleanly");
    }
    Ok(())
}

/// One shutdown signal shared by the HTTP server and the realtime consumer, so
/// the consumer deletes its Iggy groups before the process exits.
async fn stopped_signal(mut stopped: tokio::sync::watch::Receiver<bool>) {
    let _ = stopped.wait_for(|stop| *stop).await;
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
