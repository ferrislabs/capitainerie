mod catalog;
mod openapi;
mod router;

use anyhow::Context;
use capitainerie_analytics::{AnalyticsArgs, DataFusionAnalytics, Dataset};
use capitainerie_args::{
    AuthArgs, DatabaseArgs, DelegationArgs, GithubArgs, LogArgs, ObservabilityArgs, SecretsArgs,
    ServerArgs,
};
use capitainerie_auth::{AcceptedClients, ExchangeArgs, FerrisKeyVerifier};
use capitainerie_bus_iggy::IggyArgs;
use capitainerie_core::{
    CapitainerieUseCase,
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApps, GithubBases};
use capitainerie_handlers::{AppState, Delegation};
use capitainerie_handlers_integrations::{GithubIntegration, signer::StateSigner};
use capitainerie_handlers_realtime::{Gateway, Hub, consumer};
use capitainerie_runtime_rig::{LlmArgs, RigCaptain};
use clap::Parser;
use std::sync::Arc;
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
    llm: LlmArgs,
    #[command(flatten)]
    secrets: SecretsArgs,
    #[command(flatten)]
    github: GithubArgs,
    #[arg(id = "console_url", long = "console-url", env = "CONSOLE_URL")]
    console_url: Option<String>,
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
    match &args.secrets.key {
        Some(key) => {
            usecase = usecase.with_secrets_cipher(
                SecretCipher::from_base64(key).context("SECRETS_KEY is unusable")?,
            );
        }
        None => tracing::warn!(
            "SECRETS_KEY is unset: provider API keys cannot be stored; runs use the worker's default endpoint"
        ),
    }
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
    let captain_exchanger = match &args.exchange.client_id {
        Some(_) => Some(args.exchange.exchanger()?),
        None => {
            tracing::warn!("EXCHANGE_CLIENT_ID is unset: the Captain cannot act for anyone");
            None
        }
    };
    let github = github_integration(
        &args.github,
        args.console_url.clone(),
        args.secrets.key.as_deref(),
        usecase.clone(),
    )?;
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
    let app = router::router(
        state,
        realtime,
        analytics,
        catalog::build()?,
        router::Adapters {
            intake: router::IntakeAdapters {
                transcribers: catalog::build_transcribers()?,
                document_readers: catalog::build_document_readers()?,
            },
            captain: router::CaptainAdapters {
                runtime: RigCaptain::new(&args.llm).context("building the Captain runtime")?,
                exchanger: captain_exchanger,
            },
        },
        github,
        &args.server.allowed_origins,
    )?;

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

fn github_integration(
    args: &GithubArgs,
    console_url: Option<String>,
    secrets_key: Option<&str>,
    usecase: CapitainerieUseCase,
) -> anyhow::Result<GithubIntegration> {
    let apps = GithubApps::new(
        usecase,
        GithubBases {
            api_base: args.api_base.clone(),
            web_base: args.web_base.clone(),
        },
    )
    .context("building the GitHub client")?;
    let signer = secrets_key
        .map(StateSigner::from_base64)
        .transpose()
        .context("SECRETS_KEY is unusable")?;
    if signer.is_none() {
        tracing::warn!("SECRETS_KEY is unset: organizations cannot create their GitHub App");
    }
    if console_url.is_none() {
        tracing::warn!("CONSOLE_URL is unset: organizations cannot create their GitHub App");
    }
    Ok(GithubIntegration {
        apps: Arc::new(apps),
        signer,
        console_url,
    })
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

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Args;

    #[test]
    fn the_command_line_definition_is_valid() {
        Args::command().debug_assert();
    }
}
