#![allow(clippy::unwrap_used)]

use capitainerie_core::domain::agent::ports::AgentRuntime;
use capitainerie_runtime_rig::{LlmArgs, RigAgentRuntime};
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    llm: LlmArgs,
}

const TRANSCRIPT: &str = "Réunion du lundi.\nMarie doit relancer le client Acme avant vendredi.\nPaul enverra le devis révisé demain matin.";

#[tokio::test]
#[ignore = "needs a running Ollama; run with LLM_LIVE=1 and --ignored"]
async fn live_extraction_quotes_the_transcript_verbatim() {
    if std::env::var("LLM_LIVE").as_deref() != Ok("1") {
        eprintln!("skipped: LLM_LIVE is not 1");
        return;
    }
    let cli = Cli::parse_from(["live"]);
    let runtime = RigAgentRuntime::new(&cli.llm).unwrap();

    let extraction = runtime.extract(TRANSCRIPT).await.unwrap();

    assert!(!extraction.actions.is_empty(), "{extraction:?}");
    for action in &extraction.actions {
        assert!(
            TRANSCRIPT.contains(&action.quote),
            "quote is not verbatim: {action:?}"
        );
    }
    assert!(extraction.usage.input_tokens > 0);
}
