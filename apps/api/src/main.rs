mod catalog;
mod openapi;
mod router;

use anyhow::Context;
use capitainerie_analytics::{AnalyticsArgs, DataFusionAnalytics, Dataset};
use capitainerie_args::{
    AuthArgs, DatabaseArgs, DelegationArgs, GithubAppArgs, LogArgs, ObservabilityArgs, SecretsArgs,
    ServerArgs,
};
use capitainerie_auth::{AcceptedClients, ExchangeArgs, FerrisKeyVerifier};
use capitainerie_bus_iggy::IggyArgs;
use capitainerie_core::{
    CapitainerieUseCase,
    domain::agent::SecretToken,
    infrastructure::{
        authorization::{AuditedAuthorizer, LocalPolicyEngine},
        crypto::SecretCipher,
        postgres,
    },
};
use capitainerie_github::{GithubApp, GithubClient};
use capitainerie_handlers::{AppState, Delegation};
use capitainerie_handlers_integrations::{GithubIntegration, signer::StateSigner};
use capitainerie_handlers_realtime::{Gateway, Hub, consumer};
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
    secrets: SecretsArgs,
    #[command(flatten)]
    github: GithubAppArgs,
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
    let authz = AuditedAuthorizer::new(LocalPolicyEngine, pool.clone());
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
    let github = github_integration(&args.github, args.secrets.key.as_deref())?;
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
        authz,
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
    args: &GithubAppArgs,
    secrets_key: Option<&str>,
) -> anyhow::Result<GithubIntegration> {
    let any_set = args.app_id.is_some()
        || args.private_key.is_some()
        || args.client_id.is_some()
        || args.client_secret.is_some();
    let client = match (args.app_configured(), any_set) {
        (true, _) => {
            let (Some(app_id), Some(key), Some(client_id), Some(client_secret)) = (
                args.app_id,
                &args.private_key,
                &args.client_id,
                &args.client_secret,
            ) else {
                anyhow::bail!("the GitHub App settings are incomplete");
            };
            let mut app = GithubApp::new(app_id, SecretToken::new(key.as_str()));
            app.api_base.clone_from(&args.api_base);
            app.oauth_base.clone_from(&args.oauth_base);
            app.client_id = Some(client_id.clone());
            app.client_secret = Some(SecretToken::new(client_secret.as_str()));
            Some(Arc::new(
                GithubClient::new(app).context("building the GitHub client")?,
            ))
        }
        (false, false) => {
            tracing::warn!(
                "the GitHub App settings are unset: GitHub cannot be connected and cards cannot be executed"
            );
            None
        }
        (false, true) => anyhow::bail!(
            "GITHUB_APP_ID, GITHUB_APP_PRIVATE_KEY, GITHUB_APP_CLIENT_ID and GITHUB_APP_CLIENT_SECRET must be set together"
        ),
    };
    let signer = secrets_key
        .map(StateSigner::from_base64)
        .transpose()
        .context("SECRETS_KEY is unusable")?;
    Ok(GithubIntegration {
        client,
        slug: args.slug.clone(),
        signer,
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
