mod indexer;
mod orchestrator;

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use capitainerie_args::{DatabaseArgs, DelegationArgs, LogArgs, ObservabilityArgs};
use capitainerie_auth::ExchangeArgs;
use capitainerie_bus_iggy::{IggyArgs, IggyEventBus};
use capitainerie_core::{
    CapitainerieUseCase, MeetingPipeline, PipelineConfig,
    domain::{run::step::Pricing, similarity::DEFAULT_DEDUP_THRESHOLD},
    infrastructure::{
        crypto::SecretCipher,
        event::{OutboxRelay, OutboxRelayConfig},
        postgres,
    },
};
use capitainerie_runtime_rig::{EmbeddingArgs, LlmArgs, OpenAiEmbedder, RigAgentRuntime};
use capitainerie_vector_lancedb::LanceVectorStore;
use clap::Parser;
use tokio::sync::watch;

use crate::{indexer::CardIndexer, orchestrator::Orchestrator};

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
    #[command(flatten)]
    exchange: ExchangeArgs,
    #[command(flatten)]
    delegation: DelegationArgs,
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

    let pool = postgres::connect(&args.database.url, args.database.max_connections)
        .await
        .context("connecting to postgres")?;
    let key =
        args.delegation.key.as_deref().context(
            "DELEGATION_KEY is required: the worker reads the delegations the API sealed",
        )?;
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::from_base64(key)?);
    let bus = IggyEventBus::connect(&args.iggy)
        .await
        .context("connecting to iggy")?;
    let writer_agent = args
        .delegation
        .agents
        .first()
        .cloned()
        .context("PIPELINE_AGENTS names no writer agent")?;
    let embedder = OpenAiEmbedder::new(&args.llm, &args.embedding)?;
    let vectors = LanceVectorStore::open(&args.lancedb_uri, args.embedding.dimensions)
        .await
        .context("opening the vector store")?;
    let indexer = CardIndexer::new(
        usecase.clone(),
        bus.clone(),
        embedder.clone(),
        vectors.clone(),
    );
    let pipeline = MeetingPipeline::new(
        usecase,
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
        },
    );
    let orchestrator = Orchestrator::new(Arc::new(pipeline), bus.clone(), args.concurrency);

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
