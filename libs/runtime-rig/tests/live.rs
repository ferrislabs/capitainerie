#![allow(clippy::unwrap_used)]

use capitainerie_core::domain::{
    agent::{ExtractedAction, LocatedAction, ports::AgentRuntime},
    similarity::{DEFAULT_DEDUP_THRESHOLD, Embedding, SimilarCard, ports::Embedder},
};
use capitainerie_runtime_rig::{EmbeddingArgs, LlmArgs, OpenAiEmbedder, RigAgentRuntime};
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    llm: LlmArgs,
    #[command(flatten)]
    embedding: EmbeddingArgs,
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

const SAME_TASK: [(&str, &str); 5] = [
    (
        "Mettre à jour la page de prix\nRefaire la page tarifs du site\nil faut mettre à jour la page de prix",
        "Actualiser la grille tarifaire sur le site\nChanger les tarifs affichés\non doit actualiser la grille tarifaire sur le site",
    ),
    (
        "Envoyer l'email aux clients\nInformer de la nouvelle offre\nj'envoie l'email aux clients",
        "Prévenir les clients existants par email\nMessage sur la nouvelle offre\nil faut prévenir les clients existants par email",
    ),
    (
        "Relancer Acme pour le devis\nAvant vendredi\nMarie doit relancer Acme avant vendredi",
        "Rappeler le client Acme au sujet du devis\nD'ici la fin de semaine\nMarie rappelle Acme avant vendredi",
    ),
    (
        "Migrer la CI vers les runners ARM\nAvant fin octobre\nil faut migrer la CI vers les runners ARM",
        "Passer l'intégration continue sur des runners ARM\nPour fin octobre\non passe la CI sur ARM avant fin octobre",
    ),
    (
        "Préparer la démo du jeudi\nSlides et jeu de données\nje prépare la démo de jeudi",
        "Organiser la présentation de jeudi\nSupport et données de démonstration\nje m'occupe de la présentation de jeudi",
    ),
];

const RELATED_TASK: [(&str, &str); 5] = [
    (
        "Ouvrir un ticket pour le bug de facturation\nRéférencer l'incident dans le tracker\nouvre un ticket pour le bug de facturation",
        "Corriger le bug de facturation\nLes montants sont faux sur les factures\non corrige le bug de facturation",
    ),
    (
        "Envoyer l'email aux clients\nInformer de la nouvelle offre\nj'envoie l'email aux clients",
        "Rédiger le texte de l'email aux clients\nValidation par le marketing\nrédiger le texte de l'email aux clients",
    ),
    (
        "Mettre à jour la page de prix\nRefaire la page tarifs du site\nil faut mettre à jour la page de prix",
        "Mettre à jour la page de contact\nAjouter la nouvelle adresse du bureau\nil faut mettre à jour la page de contact",
    ),
    (
        "Relancer Acme pour le devis\nAvant vendredi\nMarie doit relancer Acme avant vendredi",
        "Relancer Globex pour le devis\nAvant vendredi\nPaul doit relancer Globex avant vendredi",
    ),
    (
        "Migrer la CI vers les runners ARM\nAvant fin octobre\nil faut migrer la CI vers les runners ARM",
        "Migrer la base de données vers Postgres 17\nAvant fin octobre\nil faut migrer la base vers Postgres 17",
    ),
];

fn cosine(a: &Embedding, b: &Embedding) -> f32 {
    let dot: f32 = a.0.iter().zip(&b.0).map(|(x, y)| x * y).sum();
    let norm = |v: &Embedding| v.0.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm(a) * norm(b))
}

async fn scores(embedder: &OpenAiEmbedder, pairs: &[(&str, &str)]) -> Vec<f32> {
    let texts: Vec<String> = pairs
        .iter()
        .flat_map(|(a, b)| [(*a).to_owned(), (*b).to_owned()])
        .collect();
    let (embeddings, _) = embedder.embed(&texts).await.unwrap();
    embeddings
        .chunks(2)
        .map(|pair| cosine(&pair[0], &pair[1]))
        .collect()
}

#[tokio::test]
#[ignore = "needs bge-m3 in a running Ollama; run with EMBEDDING_LIVE=1 and --ignored --nocapture"]
async fn live_threshold_separates_paraphrases_from_related_tasks() {
    if std::env::var("EMBEDDING_LIVE").as_deref() != Ok("1") {
        eprintln!("skipped: EMBEDDING_LIVE is not 1");
        return;
    }
    let cli = Cli::parse_from(["live"]);
    let embedder = OpenAiEmbedder::new(&cli.llm, &cli.embedding).unwrap();

    let same = scores(&embedder, &SAME_TASK).await;
    let related = scores(&embedder, &RELATED_TASK).await;

    eprintln!("same task:      {same:.3?}");
    eprintln!("related tasks:  {related:.3?}");
    let lowest_same = same.iter().copied().fold(f32::MAX, f32::min);
    let highest_related = related.iter().copied().fold(f32::MIN, f32::max);
    eprintln!(
        "lowest same {lowest_same:.3}, highest related {highest_related:.3}, threshold {DEFAULT_DEDUP_THRESHOLD}"
    );
    let merged_wrongly = related
        .iter()
        .filter(|s| **s >= DEFAULT_DEDUP_THRESHOLD)
        .count();
    let missed = same
        .iter()
        .filter(|s| **s < DEFAULT_DEDUP_THRESHOLD)
        .count();
    eprintln!("different tasks merged: {merged_wrongly}, paraphrases missed: {missed}");
    assert_eq!(
        merged_wrongly, 0,
        "the threshold would merge different tasks"
    );
    assert_eq!(missed, 0, "the threshold misses a paraphrase");
}

const JUDGE_PAIRS: [(&str, &str, &str, &str, bool); 8] = [
    (
        "Rédiger et envoyer l'email aux clients",
        "",
        "Envoyer un email aux clients",
        "",
        true,
    ),
    (
        "Mettre à jour la page de prix sur le site",
        "",
        "Actualiser la grille tarifaire sur le site",
        "",
        true,
    ),
    (
        "Relancer Acme",
        "Avant vendredi",
        "Relancer Globex",
        "Avant vendredi",
        false,
    ),
    (
        "Ouvrir un ticket pour le bug de facturation",
        "",
        "Corriger le bug de facturation",
        "",
        false,
    ),
    (
        "Préparer la démo de jeudi",
        "Paul s'en occupe",
        "Organiser la présentation de jeudi",
        "Slides et données",
        true,
    ),
    (
        "Migrer la CI vers les runners ARM",
        "",
        "Migrer la base de données vers Postgres 17",
        "",
        false,
    ),
    (
        "Envoyer le devis à Acme avant mardi",
        "Marie",
        "Envoyer le devis à Acme",
        "",
        true,
    ),
    (
        "Écrire la documentation de l'API",
        "",
        "Relire la documentation de l'API",
        "",
        false,
    ),
];

#[tokio::test]
#[ignore = "needs qwen2.5:7b in a running Ollama; run with JUDGE_LIVE=1 and --ignored --nocapture"]
async fn live_judge_separates_the_same_task_from_a_related_one() {
    if std::env::var("JUDGE_LIVE").as_deref() != Ok("1") {
        eprintln!("skipped: JUDGE_LIVE is not 1");
        return;
    }
    let cli = Cli::parse_from(["live"]);
    let runtime = RigAgentRuntime::new(&cli.llm).unwrap();
    let mut wrong = 0;
    for (title, description, card_title, card_description, expected) in JUDGE_PAIRS {
        let action = LocatedAction {
            action: ExtractedAction {
                title: title.into(),
                description: description.into(),
                quote: title.into(),
                owner: None,
                due: None,
            },
            start: 0,
            end: 1,
        };
        let card = SimilarCard {
            card_id: capitainerie_common::CardId::new(),
            score: 0.7,
            text: format!("{card_title}\n{card_description}"),
        };
        let started = std::time::Instant::now();
        let judgement = runtime.judge_duplicate(&action, &card).await.unwrap();
        eprintln!(
            "{title:?} vs {card_title:?}: same={} (expected {expected}) in {} ms, {}+{} tokens, reason: {}",
            judgement.same_task,
            started.elapsed().as_millis(),
            judgement.usage.input_tokens,
            judgement.usage.output_tokens,
            judgement.reason
        );
        if judgement.same_task != expected {
            wrong += 1;
        }
    }
    assert_eq!(wrong, 0, "{wrong} wrong judgement(s)");
}
