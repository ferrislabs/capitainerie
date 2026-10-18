mod dispatch;
mod indexer;
mod orchestrator;

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use capitainerie_args::{
    DatabaseArgs, DelegationArgs, ExecutorArgs, GithubArgs, LogArgs, ObservabilityArgs, SecretsArgs,
};
use capitainerie_auth::ExchangeArgs;
use capitainerie_bus_iggy::{IggyArgs, IggyEventBus};
use capitainerie_core::{
    CapitainerieUseCase, ExecuteConfig, ExecutePipeline, Executor, MeetingPipeline, PipelineConfig,
    domain::{
        provider::ModelAssignments,
        run::step::Pricing,
        similarity::{DEFAULT_DEDUP_GRAY_FLOOR, DEFAULT_DEDUP_THRESHOLD},
    },
    infrastructure::{
        crypto::SecretCipher,
        event::{OutboxRelay, OutboxRelayConfig},
        postgres,
    },
};
use capitainerie_github::{GithubApps, GithubBases};
use capitainerie_runner_kube::{KubeRunnerConfig, KubeTaskRunner};
use capitainerie_runtime_rig::{
    EmbeddingArgs, LlmArgs, RigAgentRuntime, RigEmbedder, claude_prices,
};
use capitainerie_vector_lancedb::LanceVectorStore;
use clap::Parser;
use tokio::sync::watch;

use crate::{dispatch::Dispatcher, indexer::CardIndexer, orchestrator::Orchestrator};

#[derive(Debug, Parser)]
#[command(name = "capitainerie-worker", version)]
struct Args {
    #[command(flatten)]
    database: DatabaseArgs,
    #[command(flatten)]
    iggy: IggyArgs,
    #[command(flatten)]
    llm: LlmArgs,
    #[command(flatten)]
    embedding: EmbeddingArgs,
    /// Where the card vectors live: a local directory, or an object store url.
    #[arg(
        long = "lancedb-uri",
        env = "LANCEDB_URI",
        default_value = "./data/lancedb"
    )]
    lancedb_uri: String,
    /// Cosine similarity from which an action is taken for an existing card.
    #[arg(
        long = "dedup-threshold",
        env = "DEDUP_THRESHOLD",
        default_value_t = DEFAULT_DEDUP_THRESHOLD
    )]
    dedup_threshold: f32,
    #[arg(
        long = "dedup-gray-floor",
        env = "DEDUP_GRAY_FLOOR",
        default_value_t = DEFAULT_DEDUP_GRAY_FLOOR
    )]
    dedup_gray_floor: f32,
    #[command(flatten)]
    exchange: ExchangeArgs,
    #[command(flatten)]
    delegation: DelegationArgs,
    #[command(flatten)]
    secrets: SecretsArgs,
    /// Price of the model, in micro-units per million input tokens (0 for a local model).
    #[arg(
        long = "llm-price-input",
        env = "LLM_PRICE_INPUT_PER_MTOK_MICROS",
        default_value_t = 0
    )]
    price_input: u64,
    #[arg(
        long = "llm-price-output",
        env = "LLM_PRICE_OUTPUT_PER_MTOK_MICROS",
        default_value_t = 0
    )]
    price_output: u64,
    /// How many runs execute at once.
    #[arg(
        long = "worker-concurrency",
        env = "WORKER_CONCURRENCY",
        default_value_t = 2
    )]
    concurrency: usize,
    #[command(flatten)]
    executor: ExecutorArgs,
    #[command(flatten)]
    github: GithubArgs,
    /// Base URL of the console, to link a card from its pull request.
    #[arg(long = "console-url", env = "CONSOLE_URL")]
    console_url: Option<String>,
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

    anyhow::ensure!(
        args.dedup_gray_floor <= args.dedup_threshold,
        "DEDUP_GRAY_FLOOR ({}) must not exceed DEDUP_THRESHOLD ({})",
        args.dedup_gray_floor,
        args.dedup_threshold
    );

    let pool = postgres::connect(&args.database.url, args.database.max_connections)
        .await
        .context("connecting to postgres")?;
    let key =
        args.delegation.key.as_deref().context(
            "DELEGATION_KEY is required: the worker reads the delegations the API sealed",
        )?;
    let mut usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::from_base64(key)?);
    match &args.secrets.key {
        Some(key) => {
            usecase = usecase.with_secrets_cipher(
                SecretCipher::from_base64(key).context("SECRETS_KEY is unusable")?,
            );
        }
        None => tracing::warn!(
            "SECRETS_KEY is unset: the organizations' provider keys cannot be read, every run uses the default endpoint"
        ),
    }
    let bus = IggyEventBus::connect(&args.iggy)
        .await
        .context("connecting to iggy")?;
    let writer_agent = args
        .delegation
        .agents
        .first()
        .cloned()
        .context("PIPELINE_AGENTS names no writer agent")?;
    let default_models = ModelAssignments::defaults(&args.llm.model, &args.embedding.model);
    let embedder = RigEmbedder::new(&args.llm)?;
    let vectors = LanceVectorStore::open(&args.lancedb_uri)
        .await
        .context("opening the vector store")?;
    let indexer = CardIndexer::new(
        usecase.clone(),
        bus.clone(),
        embedder.clone(),
        vectors.clone(),
        default_models.clone(),
    );
    let meeting = MeetingPipeline::new(
        usecase.clone(),
        RigAgentRuntime::new(&args.llm)?,
        args.exchange.exchanger()?,
        embedder,
        vectors,
        PipelineConfig {
            writer_agent,
            pricing: Pricing {
                input_per_million_micros: args.price_input,
                output_per_million_micros: args.price_output,
            },
            max_attempts: 3,
            stale_after: Duration::from_secs(15 * 60),
            dedup_threshold: args.dedup_threshold,
            dedup_gray_floor: args.dedup_gray_floor,
            default_models: default_models.clone(),
        },
    );
    let executor = executor_from(&args, usecase.clone()).await;
    let execution = ExecutePipeline::new(
        usecase,
        executor,
        ExecuteConfig {
            max_attempts: 3,
            retry_base: Duration::from_secs(1),
            stale_after: Duration::from_secs(15 * 60),
            max_turns: args.executor.max_turns,
            deadline: Duration::from_secs(args.executor.deadline_secs),
            progress_every: Duration::from_secs(5),
            console_url: args.console_url.clone(),
            default_models,
            prices: claude_prices(),
        },
    );
    let pipeline = Dispatcher::new(meeting, execution);
    let orchestrator = Orchestrator::new(
        Arc::new(pipeline),
        bus.clone(),
        args.concurrency,
        args.executor.concurrency,
    );

    let (stop, stopped) = watch::channel(false);
    tracing::info!(
        stream = %args.iggy.stream,
        model = %args.llm.model,
        embedding_model = %args.embedding.model,
        lancedb = %args.lancedb_uri,
        "worker started"
    );
    let relay = OutboxRelay::new(pool, bus, OutboxRelayConfig::default());
    let mut relay_stop = stopped.clone();
    let (relayed, orchestrated, (), ()) = tokio::join!(
        relay.run(async move {
            let _ = relay_stop.wait_for(|stop| *stop).await;
        }),
        orchestrator.run(stopped.clone()),
        indexer.run(stopped),
        async move {
            capitainerie_telemetry::shutdown_signal().await;
            tracing::info!("worker stopping");
            let _ = stop.send(true);
        },
    );
    relayed?;
    orchestrated?;
    tracing::info!("worker stopped");
    Ok(())
}

async fn executor_from(
    args: &Args,
    usecase: CapitainerieUseCase,
) -> Executor<KubeTaskRunner, GithubApps> {
    match build_executor(args, usecase).await {
        Ok(executor) => executor,
        Err(reason) => {
            tracing::warn!(%reason, "card executions are disabled");
            Executor::Disabled { reason }
        }
    }
}

async fn build_executor(
    args: &Args,
    usecase: CapitainerieUseCase,
) -> Result<Executor<KubeTaskRunner, GithubApps>, String> {
    let namespace = args
        .executor
        .namespace
        .as_deref()
        .ok_or("les exécutions sont désactivées : EXECUTOR_NAMESPACE n'est pas défini")?;
    let (Some(image), Some(proxy_url)) = (&args.executor.image, &args.executor.proxy_url) else {
        return Err("EXECUTOR_IMAGE et EXECUTOR_PROXY_URL sont requis pour les exécutions".into());
    };
    let mut config = KubeRunnerConfig::new(image.as_str(), proxy_url.as_str());
    config.namespace = namespace.to_owned();
    let runner = KubeTaskRunner::try_default(config)
        .await
        .map_err(|error| format!("le cluster Kubernetes est inaccessible : {error}"))?;
    let host = GithubApps::new(
        usecase,
        GithubBases {
            api_base: args.github.api_base.clone(),
            web_base: args.github.web_base.clone(),
        },
    )
    .map_err(|error| format!("le client GitHub est inutilisable : {error}"))?;
    Ok(Executor::Ready { runner, host })
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
