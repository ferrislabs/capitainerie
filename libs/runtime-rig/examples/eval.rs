use std::{
    fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use capitainerie_core::domain::agent::{ExtractedAction, ports::AgentRuntime};
use capitainerie_runtime_rig::{LlmArgs, RigAgentRuntime};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    llm: LlmArgs,
    #[arg(long)]
    only: Option<String>,
    #[arg(long)]
    label: Option<String>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    transcript: String,
    expected: Vec<Expected>,
}

#[derive(Deserialize)]
struct Expected {
    must_match: Vec<String>,
    owner: Option<String>,
    due: Option<String>,
    quote_must_contain: String,
}

#[derive(Serialize)]
struct Scored {
    name: String,
    latency_ms: u128,
    expected: usize,
    extracted: usize,
    matched: usize,
    correct: usize,
    duplicates: usize,
    owner_ok: usize,
    due_ok: usize,
    quote_ok: usize,
    anchorable: usize,
    prefixed: usize,
    quote_lengths: Vec<usize>,
    actions: Vec<ExtractedAction>,
    error: Option<String>,
}

fn fold(text: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !space {
                out.push(' ');
            }
            space = true;
            continue;
        }
        space = false;
        let c = match c {
            '\u{2018}' | '\u{2019}' | '\u{02BC}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{00AB}' | '\u{00BB}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            other => other,
        };
        out.extend(c.to_lowercase());
    }
    out.trim().to_owned()
}

fn plain(text: &str) -> String {
    fold(text)
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

fn anchorable(body: &str, quote: &str) -> bool {
    let needle = fold(quote);
    !needle.is_empty() && fold(body).contains(&needle)
}

fn has_speaker_prefix(quote: &str) -> bool {
    let Some((name, _)) = quote.trim().split_once(" : ") else {
        return false;
    };
    let words = name.split_whitespace().count();
    (1..=3).contains(&words) && name.chars().next().is_some_and(|c| c.is_uppercase())
}

fn matches(expected: &Expected, action: &ExtractedAction) -> bool {
    let haystack = plain(&format!(
        "{} {} {}",
        action.title, action.description, action.quote
    ));
    expected
        .must_match
        .iter()
        .all(|keyword| haystack.contains(&plain(keyword)))
}

fn lenient(expected: Option<&str>, got: Option<&str>) -> bool {
    match (expected, got) {
        (None, None) => true,
        (Some(expected), Some(got)) => {
            let (expected, got) = (plain(expected), plain(got));
            got.contains(&expected) || expected.contains(&got)
        }
        _ => false,
    }
}

fn score(
    case: &Case,
    actions: Vec<ExtractedAction>,
    latency_ms: u128,
    error: Option<String>,
) -> Scored {
    let mut claimed = vec![false; case.expected.len()];
    let mut matched = 0;
    let mut correct = 0;
    let mut duplicates = 0;
    let (mut owner_ok, mut due_ok, mut quote_ok) = (0, 0, 0);
    for action in &actions {
        let Some(index) = case
            .expected
            .iter()
            .position(|expected| matches(expected, action))
        else {
            continue;
        };
        correct += 1;
        if claimed[index] {
            duplicates += 1;
            continue;
        }
        claimed[index] = true;
        matched += 1;
        let expected = &case.expected[index];
        owner_ok += usize::from(lenient(expected.owner.as_deref(), action.owner.as_deref()));
        due_ok += usize::from(lenient(expected.due.as_deref(), action.due.as_deref()));
        quote_ok +=
            usize::from(plain(&action.quote).contains(&plain(&expected.quote_must_contain)));
    }
    Scored {
        name: case.name.clone(),
        latency_ms,
        expected: case.expected.len(),
        extracted: actions.len(),
        matched,
        correct,
        duplicates,
        owner_ok,
        due_ok,
        quote_ok,
        anchorable: actions
            .iter()
            .filter(|action| anchorable(&case.transcript, &action.quote))
            .count(),
        prefixed: actions
            .iter()
            .filter(|action| has_speaker_prefix(&action.quote))
            .count(),
        quote_lengths: actions
            .iter()
            .map(|action| action.quote.chars().count())
            .collect(),
        actions,
        error,
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "  - ".into()
    } else {
        format!("{:3.0}%", 100.0 * part as f64 / whole as f64)
    }
}

fn median(values: &mut [usize]) -> usize {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or(0)
}

fn load(dir: &Path, only: Option<&str>) -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    let mut cases = Vec::new();
    for path in paths {
        let case: Case = serde_json::from_str(&fs::read_to_string(&path)?)?;
        if only.is_none_or(|only| case.name.contains(only)) {
            cases.push(case);
        }
    }
    Ok(cases)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("eval");
    let cases = load(&root.join("data"), cli.only.as_deref())?;
    let runtime = RigAgentRuntime::new(&cli.llm)?;

    let mut results = Vec::new();
    for case in &cases {
        let started = Instant::now();
        let outcome = runtime.extract(&case.transcript).await;
        let latency = started.elapsed().as_millis();
        let scored = match outcome {
            Ok(extraction) => score(case, extraction.actions, latency, None),
            Err(error) => score(case, Vec::new(), latency, Some(error.to_string())),
        };
        eprintln!("done {} in {:.1}s", scored.name, latency as f64 / 1000.0);
        results.push(scored);
    }

    println!(
        "{:<22} {:>3} {:>3} {:>3} {:>3} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>4} {:>7}",
        "transcript",
        "exp",
        "got",
        "ok",
        "dup",
        "rec",
        "prec",
        "own",
        "due",
        "quo",
        "anch",
        "pfx",
        "sec"
    );
    for r in &results {
        println!(
            "{:<22} {:>3} {:>3} {:>3} {:>3} {} {} {} {} {} {} {:>4} {:>7.1}{}",
            r.name,
            r.expected,
            r.extracted,
            r.matched,
            r.duplicates,
            percent(r.matched, r.expected),
            percent(r.correct, r.extracted),
            percent(r.owner_ok, r.matched),
            percent(r.due_ok, r.matched),
            percent(r.quote_ok, r.matched),
            percent(r.anchorable, r.extracted),
            r.prefixed,
            r.latency_ms as f64 / 1000.0,
            r.error
                .as_deref()
                .map(|e| format!("  ERROR {e}"))
                .unwrap_or_default(),
        );
    }
    let sum = |f: fn(&Scored) -> usize| results.iter().map(f).sum::<usize>();
    let (expected, extracted, matched) = (
        sum(|r| r.expected),
        sum(|r| r.extracted),
        sum(|r| r.matched),
    );
    let mut lengths: Vec<usize> = results
        .iter()
        .flat_map(|r| r.quote_lengths.clone())
        .collect();
    let mean_latency = results.iter().map(|r| r.latency_ms).sum::<u128>() as f64
        / results.len().max(1) as f64
        / 1000.0;
    println!(
        "{:<22} {:>3} {:>3} {:>3} {:>3} {} {} {} {} {} {} {:>4} {:>7.1}",
        "TOTAL",
        expected,
        extracted,
        matched,
        sum(|r| r.duplicates),
        percent(matched, expected),
        percent(sum(|r| r.correct), extracted),
        percent(sum(|r| r.owner_ok), matched),
        percent(sum(|r| r.due_ok), matched),
        percent(sum(|r| r.quote_ok), matched),
        percent(sum(|r| r.anchorable), extracted),
        sum(|r| r.prefixed),
        mean_latency,
    );
    println!("median quote length: {} chars", median(&mut lengths));

    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let out = serde_json::to_string_pretty(&serde_json::json!({
        "model": cli.llm.model,
        "timestamp": stamp,
        "median_quote_len": median(&mut lengths),
        "mean_latency_s": mean_latency,
        "results": results,
    }))?;
    let dir = root.join("results");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(format!("{stamp}.json")), &out)?;
    if let Some(label) = cli.label {
        fs::write(dir.join(format!("{label}.json")), &out)?;
    }
    Ok(())
}
