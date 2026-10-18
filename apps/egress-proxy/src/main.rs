use anyhow::Context;
use capitainerie_core::{
    CapitainerieUseCase,
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_egress_proxy::{Live, Proxy, ProxyConfig, config::Args, router};
use capitainerie_github::{GithubApps, GithubBases};
use clap::Parser;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    let _telemetry =
        capitainerie_telemetry::init("capitainerie-egress-proxy", &args.log, &args.observability)?;

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
            "SECRETS_KEY is unset: the organizations' Anthropic keys cannot be read, every Anthropic call is refused"
        ),
    }
    let github = GithubApps::new(
        usecase.clone(),
        GithubBases {
            api_base: args.github.api_base.clone(),
            web_base: args.github.web_base.clone(),
        },
    )
    .context("building the GitHub client")?;

    let proxy = Proxy::new(
        Live::new(usecase, github),
        ProxyConfig {
            anthropic_base: args.proxy.anthropic_base.clone(),
            github_base: args.proxy.github_base.clone(),
        },
    )
    .context("building the upstream client")?;

    let addr = format!("{}:{}", args.proxy.host, args.proxy.port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "egress-proxy listening");
    axum::serve(listener, router(proxy))
        .with_graceful_shutdown(capitainerie_telemetry::shutdown_signal())
        .await?;
    Ok(())
}
