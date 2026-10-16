#![allow(clippy::unwrap_used)]

mod common;

use capitainerie_common::{CardId, CoreError, RunId, TranscriptId};
use capitainerie_core::domain::{
    agent::{
        DuplicateAction, ExtractedAction, LocatedAction, SecretToken, WriteRequest,
        ports::AgentRuntime,
    },
    similarity::{Embedding, ports::Embedder},
};
use capitainerie_runtime_rig::{EmbeddingArgs, OpenAiEmbedder, RigAgentRuntime};
use common::{FakeLlm, FakeMcp, MODEL, args, text_response, tool_call_response};
use serde_json::json;
use uuid::Uuid;

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

fn submit(actions: serde_json::Value) -> serde_json::Value {
    json!({"actions": actions})
}

fn located(title: &str, quote: &str, start: u32, end: u32) -> LocatedAction {
    LocatedAction {
        action: ExtractedAction {
            title: title.into(),
            description: "ctx".into(),
            quote: quote.into(),
            owner: Some("Marie".into()),
            due: None,
        },
        start,
        end,
    }
}

fn write_request(actions: Vec<LocatedAction>) -> WriteRequest {
    WriteRequest {
        run_id: RunId(Uuid::from_u128(11)),
        transcript_id: TranscriptId(Uuid::from_u128(22)),
        actions,
        duplicates: Vec::new(),
        mcp_token: SecretToken::new("secret-token"),
        traceparent: Some(TRACEPARENT.to_owned()),
    }
}

#[tokio::test]
async fn extract_returns_the_actions_and_the_usage() {
    let llm = FakeLlm::with(vec![tool_call_response(
        "submit",
        &submit(json!([
            {"title": "Relancer Acme", "description": "Avant vendredi", "quote": "relancer Acme", "owner": "Marie", "due": "vendredi"},
            {"title": "Envoyer le devis", "description": "", "quote": "envoyer le devis"}
        ])),
        120,
        30,
    )]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, "http://127.0.0.1:1/mcp")).unwrap();

    let extraction = runtime
        .extract("Marie doit relancer Acme. Il faut envoyer le devis.")
        .await
        .unwrap();

    assert_eq!(extraction.actions.len(), 2);
    assert_eq!(extraction.actions[0].title, "Relancer Acme");
    assert_eq!(extraction.actions[0].owner.as_deref(), Some("Marie"));
    assert_eq!(extraction.actions[0].due.as_deref(), Some("vendredi"));
    assert_eq!(extraction.actions[1].owner, None);
    assert_eq!(extraction.usage.model, MODEL);
    assert_eq!(extraction.usage.input_tokens, 120);
    assert_eq!(extraction.usage.output_tokens, 30);
    let request = &llm.request_bodies()[0];
    assert!(request.contains("Marie doit relancer Acme"));
    assert!(request.contains("character for character"));
}

#[tokio::test]
async fn extract_retries_malformed_output_and_sums_usage() {
    let llm = FakeLlm::with(vec![
        text_response("je ne sais pas", 100, 10),
        tool_call_response(
            "submit",
            &submit(json!([{"title": "T", "description": "D", "quote": "Q"}])),
            110,
            20,
        ),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, "http://127.0.0.1:1/mcp")).unwrap();

    let extraction = runtime.extract("transcript").await.unwrap();

    assert_eq!(extraction.actions.len(), 1);
    assert_eq!(extraction.usage.input_tokens, 210);
    assert_eq!(extraction.usage.output_tokens, 30);
    assert_eq!(llm.request_bodies().len(), 2);
}

#[tokio::test]
async fn extract_gives_up_as_invalid_after_two_retries() {
    let llm = FakeLlm::with(vec![
        text_response("a", 1, 1),
        text_response("b", 1, 1),
        text_response("c", 1, 1),
        text_response("d", 1, 1),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, "http://127.0.0.1:1/mcp")).unwrap();

    let error = runtime.extract("transcript").await.unwrap_err();

    assert!(matches!(error, CoreError::Invalid(_)), "{error:?}");
    assert_eq!(llm.request_bodies().len(), 3);
}

#[tokio::test]
async fn extract_reports_an_unreachable_model_as_internal() {
    let runtime =
        RigAgentRuntime::new(&args("http://127.0.0.1:1/v1", "http://127.0.0.1:1/mcp")).unwrap();

    let error = runtime.extract("transcript").await.unwrap_err();

    assert!(matches!(&error, CoreError::Internal(message) if message.contains("language model")));
}

#[tokio::test]
async fn write_creates_the_card_with_provenance_bearer_and_traceparent() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![
        tool_call_response(
            "propose_card",
            &json!({"action_index": 1, "title": "Envoyer le devis", "description": "Pour Marie"}),
            200,
            40,
        ),
        text_response("done", 260, 2),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();

    let report = runtime
        .write(write_request(vec![
            located("Relancer Acme", "relancer Acme", 5, 18),
            located("Envoyer le devis", "envoyer le devis", 30, 46),
        ]))
        .await
        .unwrap();

    let recorded = mcp.recorded();
    assert_eq!(recorded.len(), 1);
    let call = &recorded[0];
    assert_eq!(call.authorization.as_deref(), Some("Bearer secret-token"));
    assert_eq!(call.traceparent.as_deref(), Some(TRACEPARENT));
    assert_eq!(call.arguments["title"], "Envoyer le devis");
    assert_eq!(call.arguments["description"], "Pour Marie");
    assert_eq!(call.arguments["run_id"], Uuid::from_u128(11).to_string());
    assert_eq!(
        call.arguments["provenance"],
        json!({
            "transcript_id": Uuid::from_u128(22).to_string(),
            "start": 30,
            "end": 46,
            "quote": "envoyer le devis",
        })
    );
    assert_eq!(report.tool_calls.len(), 1);
    assert_eq!(report.tool_calls[0].tool, "create_card");
    assert!(report.tool_calls[0].succeeded);
    assert!(report.tool_calls[0].summary.contains("card-1"));
    assert!(report.tool_calls[0].summary.contains("Envoyer le devis"));
    assert_eq!(report.usage.model, MODEL);
    assert_eq!(report.usage.input_tokens, 460);
    assert_eq!(report.usage.output_tokens, 42);
    let first_request = &llm.request_bodies()[0];
    assert!(first_request.contains("propose_card"));
    assert!(first_request.contains("0. title: Relancer Acme"));
    assert!(!first_request.contains("secret-token"));
}

#[tokio::test]
async fn write_without_traceparent_sends_none() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![
        tool_call_response(
            "propose_card",
            &json!({"action_index": 0, "title": "T", "description": "D"}),
            1,
            1,
        ),
        text_response("done", 1, 1),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();
    let mut request = write_request(vec![located("A", "a", 0, 1)]);
    request.traceparent = None;

    runtime.write(request).await.unwrap();

    assert_eq!(mcp.recorded()[0].traceparent, None);
}

#[tokio::test]
async fn write_out_of_range_index_is_a_tool_error_and_no_mcp_call() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![
        tool_call_response(
            "propose_card",
            &json!({"action_index": 5, "title": "T", "description": "D"}),
            10,
            5,
        ),
        text_response("done", 20, 1),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();

    let report = runtime
        .write(write_request(vec![located("A", "a", 0, 1)]))
        .await
        .unwrap();

    assert!(mcp.recorded().is_empty());
    assert!(report.tool_calls.is_empty());
    assert_eq!(report.usage.input_tokens, 30);
    let bodies = llm.request_bodies();
    assert_eq!(bodies.len(), 2);
    assert!(bodies[1].contains("does not exist"), "{}", bodies[1]);
}

#[tokio::test]
async fn write_records_an_mcp_refusal_and_shows_it_to_the_model() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![
        tool_call_response(
            "propose_card",
            &json!({"action_index": 0, "title": "REFUSE", "description": "D"}),
            10,
            5,
        ),
        text_response("done", 20, 1),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();

    let report = runtime
        .write(write_request(vec![located("A", "a", 0, 1)]))
        .await
        .unwrap();

    assert_eq!(mcp.recorded().len(), 1);
    assert_eq!(report.tool_calls.len(), 1);
    assert!(!report.tool_calls[0].succeeded);
    assert!(report.tool_calls[0].summary.contains("forbidden"));
    assert!(llm.request_bodies()[1].contains("forbidden"));
}

#[tokio::test]
async fn write_reports_what_happened_when_the_model_never_calls_the_tool() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![text_response("done", 50, 1)]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();

    let report = runtime
        .write(write_request(vec![located("A", "a", 0, 1)]))
        .await
        .unwrap();

    assert!(mcp.recorded().is_empty());
    assert!(report.tool_calls.is_empty());
}

#[tokio::test]
async fn write_reports_an_unreachable_mcp_server_as_internal() {
    let llm = FakeLlm::with(vec![]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, "http://127.0.0.1:1/mcp")).unwrap();

    let error = runtime
        .write(write_request(vec![located("A", "a", 0, 1)]))
        .await
        .unwrap_err();

    assert!(matches!(&error, CoreError::Internal(message) if message.contains("MCP")));
    assert!(llm.request_bodies().is_empty());
}

#[tokio::test]
async fn write_proposing_the_same_action_twice_creates_one_card() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![
        tool_call_response(
            "propose_card",
            &json!({"action_index": 0, "title": "Envoyer un email", "description": "D"}),
            10,
            5,
        ),
        tool_call_response(
            "propose_card",
            &json!({"action_index": 0, "title": "Prévenir les clients", "description": "D"}),
            10,
            5,
        ),
        text_response("done", 20, 1),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();

    let report = runtime
        .write(write_request(vec![located("A", "a", 0, 1)]))
        .await
        .unwrap();

    assert_eq!(mcp.recorded().len(), 1);
    assert_eq!(report.tool_calls.len(), 1);
    assert!(llm.request_bodies()[2].contains("already has its card"));
}

fn duplicate(quote: &str, card: u128) -> DuplicateAction {
    DuplicateAction {
        action: located("Mettre à jour les prix", quote, 0, 10),
        card_id: CardId(Uuid::from_u128(card)),
        score: 0.93,
    }
}

#[tokio::test]
async fn duplicates_only_comment_the_existing_cards_and_never_call_the_model() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();
    let mut request = write_request(Vec::new());
    request.duplicates = vec![duplicate("mettre à jour les prix", 7)];

    let report = runtime.write(request).await.unwrap();

    assert!(llm.request_bodies().is_empty(), "no LLM call expected");
    assert!(mcp.recorded().is_empty(), "no card is created");
    let comments = mcp.recorded_comments();
    assert_eq!(comments.len(), 1);
    assert_eq!(
        comments[0].authorization.as_deref(),
        Some("Bearer secret-token")
    );
    assert_eq!(comments[0].traceparent.as_deref(), Some(TRACEPARENT));
    assert_eq!(
        comments[0].arguments["card_id"],
        Uuid::from_u128(7).to_string()
    );
    assert_eq!(
        comments[0].arguments["run_id"],
        Uuid::from_u128(11).to_string()
    );
    assert_eq!(
        comments[0].arguments["body"],
        format!(
            "Mentionné à nouveau dans la réunion du transcript {} : “mettre à jour les prix”",
            Uuid::from_u128(22)
        )
    );
    assert_eq!(report.tool_calls.len(), 1);
    assert_eq!(report.tool_calls[0].tool, "comment");
    assert!(report.tool_calls[0].succeeded);
    assert_eq!(report.usage.input_tokens, 0);
}

#[tokio::test]
async fn duplicates_are_commented_before_the_writer_proposes_the_new_cards() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![
        tool_call_response(
            "propose_card",
            &json!({"action_index": 0, "title": "Envoyer le devis", "description": "D"}),
            10,
            5,
        ),
        text_response("done", 20, 1),
    ]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();
    let mut request = write_request(vec![located("Envoyer le devis", "envoyer", 0, 7)]);
    request.duplicates = vec![duplicate("q1", 7), duplicate("q2", 8)];

    let report = runtime.write(request).await.unwrap();

    assert_eq!(mcp.recorded_comments().len(), 2);
    assert_eq!(mcp.recorded().len(), 1);
    let tools: Vec<_> = report.tool_calls.iter().map(|c| c.tool.as_str()).collect();
    assert_eq!(tools, ["comment", "comment", "create_card"]);
    assert!(!llm.request_bodies()[0].contains("Mettre à jour les prix"));
}

#[tokio::test]
async fn a_refused_comment_is_reported_as_failed() {
    let (_mcp, mcp_url) = FakeMcp::serve().await;
    let llm = FakeLlm::with(vec![]);
    let base = llm.serve().await;
    let runtime = RigAgentRuntime::new(&args(&base, &mcp_url)).unwrap();
    let mut request = write_request(Vec::new());
    request.duplicates = vec![duplicate("REFUSE", 7)];

    let report = runtime.write(request).await.unwrap();

    assert_eq!(report.tool_calls.len(), 1);
    assert!(!report.tool_calls[0].succeeded);
    assert!(report.tool_calls[0].summary.contains("forbidden"));
}

fn embedder(base: &str) -> OpenAiEmbedder {
    OpenAiEmbedder::new(
        &args(base, "http://127.0.0.1:1/mcp"),
        &EmbeddingArgs {
            model: "bge-test".into(),
            dimensions: 4,
        },
    )
    .unwrap()
}

#[tokio::test]
async fn embeddings_come_back_in_input_order_with_the_token_count() {
    let llm = FakeLlm::with(vec![]);
    let base = llm.serve().await;

    let (embeddings, usage) = embedder(&base)
        .embed(&["un".to_owned(), "deux".to_owned()])
        .await
        .unwrap();

    assert_eq!(embeddings[0], Embedding(vec![0.0, 1.0, 0.0, 0.5]));
    assert_eq!(embeddings[1], Embedding(vec![1.0, 1.0, 0.0, 0.5]));
    assert_eq!(usage.model, "bge-test");
    assert_eq!(usage.input_tokens, 14);
    let sent = &llm.embedding_requests.lock().unwrap()[0];
    assert_eq!(sent["model"], "bge-test");
    assert_eq!(sent["input"], json!(["un", "deux"]));
}

#[tokio::test]
async fn embeddings_of_the_wrong_size_are_refused() {
    let llm = FakeLlm::with(vec![]);
    let base = llm.serve().await;
    let wrong = OpenAiEmbedder::new(
        &args(&base, "http://127.0.0.1:1/mcp"),
        &EmbeddingArgs {
            model: "bge-test".into(),
            dimensions: 1024,
        },
    )
    .unwrap();

    let error = wrong.embed(&["un".to_owned()]).await.unwrap_err();

    assert!(
        matches!(&error, CoreError::Internal(m) if m.contains("dimensions")),
        "{error:?}"
    );
}

#[tokio::test]
async fn embedding_nothing_makes_no_call() {
    let llm = FakeLlm::with(vec![]);
    let base = llm.serve().await;

    let (embeddings, _) = embedder(&base).embed(&[]).await.unwrap();

    assert!(embeddings.is_empty());
    assert!(llm.embedding_requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_unreachable_embedding_server_is_internal() {
    let error = embedder("http://127.0.0.1:1/v1")
        .embed(&["un".to_owned()])
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Internal(_)));
}
