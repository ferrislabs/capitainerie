#![allow(clippy::unwrap_used)]

mod common;

use capitainerie_common::{CardId, CoreError, RunId, TranscriptId};
use capitainerie_core::domain::{
    agent::{ExtractedAction, LocatedAction, SecretToken, WriteRequest, ports::AgentRuntime},
    provider::{ModelTarget, ProviderAccess, ProviderKind, ports::ModelCatalog},
    similarity::{SimilarCard, ports::Embedder},
};
use capitainerie_runtime_rig::{RigAgentRuntime, RigEmbedder, RigModelCatalog};
use common::{
    FakeMcp, access, args,
    providers::{FakeProvider, error_body, message, refusal, text_message, tool_use},
    target, tool_call_response,
};
use serde_json::{Value, json};
use uuid::Uuid;

const KEY: &str = "sk-ant-test-0123456789";
const HAIKU: &str = "claude-haiku-5-5";

fn runtime() -> RigAgentRuntime {
    RigAgentRuntime::new(&args("http://127.0.0.1:1/v1", "http://127.0.0.1:1/mcp")).unwrap()
}

fn claude(base: &str) -> ModelTarget {
    target(
        ProviderKind::Anthropic,
        HAIKU,
        Some(access(ProviderKind::Anthropic, KEY, base)),
    )
}

fn located(title: &str, quote: &str, start: u32, end: u32) -> LocatedAction {
    LocatedAction {
        action: ExtractedAction {
            title: title.into(),
            description: "ctx".into(),
            quote: quote.into(),
            owner: None,
            due: None,
        },
        start,
        end,
    }
}

fn write_request(mcp_token: &str, actions: Vec<LocatedAction>) -> WriteRequest {
    WriteRequest {
        run_id: RunId(Uuid::from_u128(11)),
        transcript_id: TranscriptId(Uuid::from_u128(22)),
        actions,
        duplicates: Vec::new(),
        mcp_token: SecretToken::new(mcp_token),
        traceparent: None,
    }
}

fn assert_claude_constraints(body: &Value) {
    for forbidden in ["temperature", "top_p", "top_k", "thinking"] {
        assert!(body.get(forbidden).is_none(), "{forbidden} in {body}");
    }
    assert!(
        body["messages"]
            .as_array()
            .unwrap()
            .last()
            .is_some_and(|last| last["role"] == "user"),
        "the conversation must not end on an assistant prefill: {body}"
    );
    if let Some(choice) = body.get("tool_choice") {
        assert_eq!(choice["type"], "auto", "forced tool choice in {body}");
    }
}

fn assert_claude_auth(headers: &std::collections::HashMap<String, String>) {
    assert_eq!(headers.get("x-api-key").map(String::as_str), Some(KEY));
    assert_eq!(
        headers.get("anthropic-version").map(String::as_str),
        Some("2023-06-01")
    );
}

fn assert_claude_headers(headers: &std::collections::HashMap<String, String>) {
    assert_claude_auth(headers);
    assert_eq!(
        headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
}

#[tokio::test]
async fn claude_extracts_through_structured_output_without_sampling_or_forced_tools() {
    let provider = FakeProvider::with(vec![text_message(
        &json!({"actions": [
            {"title": "Relancer Acme", "description": "d", "quote": "relancer Acme", "owner": "Marie", "due": "vendredi"},
            {"title": "Envoyer le devis", "description": "", "quote": "envoyer le devis", "owner": "", "due": ""}
        ]})
        .to_string(),
        120,
        30,
    )]);
    let base = provider.serve().await;

    let extraction = runtime()
        .extract(
            &claude(&base),
            "Marie doit relancer Acme. Il faut envoyer le devis.",
        )
        .await
        .unwrap();

    assert_eq!(extraction.actions.len(), 2);
    assert_eq!(extraction.actions[0].owner.as_deref(), Some("Marie"));
    assert_eq!(extraction.actions[1].owner, None);
    assert_eq!(extraction.actions[1].due, None);
    assert_eq!(extraction.usage.model, HAIKU);
    assert_eq!(extraction.usage.input_tokens, 120);
    assert_eq!(extraction.usage.output_tokens, 30);
    let requests = provider.recorded();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/v1/messages");
    assert_claude_headers(&requests[0].headers);
    let body = &requests[0].body;
    assert_claude_constraints(body);
    assert_eq!(body["model"], HAIKU);
    assert!(body["max_tokens"].as_u64().unwrap() > 0);
    assert!(body.get("tools").is_none());
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");
    assert_eq!(
        body["output_config"]["format"]["schema"]["additionalProperties"],
        false
    );
    assert!(
        body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Marie doit relancer Acme")
    );
    assert!(
        body["system"]
            .as_str()
            .unwrap()
            .contains("character for character")
    );
}

#[tokio::test]
async fn claude_retries_an_unreadable_answer_and_sums_the_usage() {
    let provider = FakeProvider::with(vec![
        text_message("not json", 100, 10),
        text_message(
            &json!({"actions": [{"title": "T", "description": "D", "quote": "Q", "owner": "", "due": ""}]}).to_string(),
            110,
            20,
        ),
    ]);
    let base = provider.serve().await;

    let extraction = runtime()
        .extract(&claude(&base), "transcript")
        .await
        .unwrap();

    assert_eq!(extraction.actions.len(), 1);
    assert_eq!(extraction.usage.input_tokens, 210);
    assert_eq!(extraction.usage.output_tokens, 30);
    assert_eq!(provider.recorded().len(), 2);
}

#[tokio::test]
async fn claude_gives_up_as_invalid_when_it_never_returns_actions() {
    let provider = FakeProvider::with(vec![
        text_message("a", 1, 1),
        text_message("b", 1, 1),
        text_message("c", 1, 1),
    ]);
    let base = provider.serve().await;

    let error = runtime()
        .extract(&claude(&base), "transcript")
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Invalid(_)), "{error:?}");
    assert_eq!(provider.recorded().len(), 3);
}

#[tokio::test]
async fn a_refusal_is_a_step_error_that_names_the_category_and_is_not_retried() {
    let provider = FakeProvider::with(vec![refusal("cyber")]);
    let base = provider.serve().await;

    let error = runtime()
        .extract(&claude(&base), "transcript")
        .await
        .unwrap_err();

    assert!(
        matches!(&error, CoreError::Invalid(message)
            if message.contains("refused") && message.contains("cyber")),
        "{error:?}"
    );
    assert_eq!(provider.recorded().len(), 1);
}

#[tokio::test]
async fn a_rejected_key_is_invalid_and_the_error_never_shows_the_key() {
    let provider = FakeProvider::with_statuses(vec![(
        401,
        error_body("authentication_error", "invalid x-api-key"),
    )]);
    let base = provider.serve().await;

    let error = runtime()
        .extract(&claude(&base), "transcript")
        .await
        .unwrap_err();

    let CoreError::Invalid(message) = &error else {
        panic!("{error:?}");
    };
    assert!(
        message.contains("401") && message.contains("API key"),
        "{message}"
    );
    assert!(!message.contains(KEY));
}

#[tokio::test]
async fn an_overloaded_claude_is_a_transient_error() {
    let provider =
        FakeProvider::with_statuses(vec![(529, error_body("overloaded_error", "Overloaded"))]);
    let base = provider.serve().await;

    let error = runtime()
        .extract(&claude(&base), "transcript")
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Internal(_)), "{error:?}");
}

#[tokio::test]
async fn claude_judges_with_a_json_schema_and_no_temperature() {
    let provider = FakeProvider::with(vec![text_message(
        r#"{"same_task": true, "reason": "Même email."}"#,
        80,
        12,
    )]);
    let base = provider.serve().await;
    let card = SimilarCard {
        card_id: CardId::new(),
        score: 0.7,
        text: "Envoyer un email aux clients".into(),
    };

    let judgement = runtime()
        .judge_duplicate(
            &claude(&base),
            &located("Rédiger et envoyer l'email aux clients", "q", 0, 1),
            &card,
        )
        .await
        .unwrap();

    assert!(judgement.same_task);
    assert_eq!(judgement.reason, "Même email.");
    assert_eq!(judgement.usage.model, HAIKU);
    let request = &provider.recorded()[0];
    assert_claude_headers(&request.headers);
    assert_claude_constraints(&request.body);
    assert_eq!(
        request.body["output_config"]["format"]["type"],
        "json_schema"
    );
    assert!(
        request
            .body
            .to_string()
            .contains("Envoyer un email aux clients")
    );
}

#[tokio::test]
async fn claude_writes_with_a_manual_tool_loop_over_the_mcp_server() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let provider = FakeProvider::with(vec![
        message(
            json!([
                {"type": "text", "text": "Je propose les cartes."},
                tool_use("toolu_1", "propose_card", &json!({"action_index": 1, "title": "Envoyer le devis", "description": "Pour Marie"})),
                tool_use("toolu_2", "propose_card", &json!({"action_index": 0, "title": "Relancer Acme", "description": "Avant vendredi"})),
            ]),
            "tool_use",
            200,
            40,
        ),
        text_message("done", 260, 2),
    ]);
    let base = provider.serve().await;
    let runtime = RigAgentRuntime::new(&args("http://127.0.0.1:1/v1", &mcp_url)).unwrap();

    let report = runtime
        .write(
            &claude(&base),
            write_request(
                "mcp-secret",
                vec![
                    located("Relancer Acme", "relancer Acme", 5, 18),
                    located("Envoyer le devis", "envoyer le devis", 30, 46),
                ],
            ),
        )
        .await
        .unwrap();

    let created = mcp.recorded();
    assert_eq!(created.len(), 2);
    assert_eq!(created[0].arguments["title"], "Envoyer le devis");
    assert_eq!(
        created[0].authorization.as_deref(),
        Some("Bearer mcp-secret")
    );
    assert_eq!(
        created[0].arguments["provenance"],
        json!({
            "transcript_id": Uuid::from_u128(22).to_string(),
            "start": 30,
            "end": 46,
            "quote": "envoyer le devis",
        })
    );
    assert_eq!(created[1].arguments["title"], "Relancer Acme");
    assert_eq!(report.tool_calls.len(), 2);
    assert!(report.tool_calls.iter().all(|call| call.succeeded));
    assert_eq!(report.usage.model, HAIKU);
    assert_eq!(report.usage.input_tokens, 460);
    assert_eq!(report.usage.output_tokens, 42);

    let requests = provider.recorded();
    assert_eq!(requests.len(), 2);
    let first = &requests[0].body;
    assert_claude_headers(&requests[0].headers);
    assert_claude_constraints(first);
    assert_eq!(first["tools"][0]["name"], "propose_card");
    assert_eq!(first["tools"][0]["strict"], true);
    assert_eq!(first["tool_choice"], json!({"type": "auto"}));
    assert!(first["system"].as_str().unwrap().contains("propose_card"));
    assert!(
        first["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("0. title: Relancer Acme")
    );
    assert!(!first.to_string().contains("mcp-secret"));

    let second = &requests[1].body;
    assert_claude_constraints(second);
    let messages = second["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"][1]["id"], "toolu_1");
    assert_eq!(messages[2]["role"], "user");
    let results = messages[2]["content"].as_array().unwrap();
    assert_eq!(results.len(), 2, "all the results come back in one message");
    assert_eq!(results[0]["type"], "tool_result");
    assert_eq!(results[0]["tool_use_id"], "toolu_1");
    assert_eq!(results[1]["tool_use_id"], "toolu_2");
    assert!(results[0]["content"].as_str().unwrap().contains("card-1"));
    assert_eq!(results[0]["is_error"], false);
}

#[tokio::test]
async fn claude_sees_the_errors_of_its_tool_calls() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let provider = FakeProvider::with(vec![
        message(
            json!([
                tool_use(
                    "toolu_1",
                    "propose_card",
                    &json!({"action_index": 9, "title": "T", "description": "D"})
                ),
                tool_use("toolu_2", "propose_card", &json!({"title": "no index"})),
                tool_use("toolu_3", "something_else", &json!({})),
            ]),
            "tool_use",
            10,
            5,
        ),
        text_message("done", 20, 1),
    ]);
    let base = provider.serve().await;
    let runtime = RigAgentRuntime::new(&args("http://127.0.0.1:1/v1", &mcp_url)).unwrap();

    let report = runtime
        .write(
            &claude(&base),
            write_request("t", vec![located("A", "a", 0, 1)]),
        )
        .await
        .unwrap();

    assert!(mcp.recorded().is_empty());
    assert!(report.tool_calls.is_empty());
    let results = provider.recorded()[1].body["messages"][2]["content"].clone();
    let results = results.as_array().unwrap();
    assert!(results.iter().all(|result| result["is_error"] == true));
    assert!(
        results[0]["content"]
            .as_str()
            .unwrap()
            .contains("does not exist")
    );
    assert!(
        results[1]["content"]
            .as_str()
            .unwrap()
            .contains("invalid arguments")
    );
    assert!(
        results[2]["content"]
            .as_str()
            .unwrap()
            .contains("unknown tool")
    );
}

#[tokio::test]
async fn claude_may_answer_without_calling_the_tool() {
    let (mcp, mcp_url) = FakeMcp::serve().await;
    let provider = FakeProvider::with(vec![text_message("done", 50, 1)]);
    let base = provider.serve().await;
    let runtime = RigAgentRuntime::new(&args("http://127.0.0.1:1/v1", &mcp_url)).unwrap();

    let report = runtime
        .write(
            &claude(&base),
            write_request("t", vec![located("A", "a", 0, 1)]),
        )
        .await
        .unwrap();

    assert!(mcp.recorded().is_empty());
    assert!(report.tool_calls.is_empty());
    assert_eq!(report.usage.input_tokens, 50);
}

#[tokio::test]
async fn a_refusal_while_writing_fails_the_step() {
    let (_mcp, mcp_url) = FakeMcp::serve().await;
    let provider = FakeProvider::with(vec![refusal("bio")]);
    let base = provider.serve().await;
    let runtime = RigAgentRuntime::new(&args("http://127.0.0.1:1/v1", &mcp_url)).unwrap();

    let error = runtime
        .write(
            &claude(&base),
            write_request("t", vec![located("A", "a", 0, 1)]),
        )
        .await
        .unwrap_err();

    assert!(
        matches!(&error, CoreError::Invalid(m) if m.contains("bio")),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_model_of_an_unconfigured_provider_is_refused_before_any_call() {
    let target = target(ProviderKind::Mistral, "mistral-large-latest", None);

    let error = runtime().extract(&target, "transcript").await.unwrap_err();

    assert!(
        matches!(&error, CoreError::Invalid(m) if m.contains("no mistral key configured")),
        "{error:?}"
    );
}

#[tokio::test]
async fn mistral_chats_through_its_own_endpoint_with_the_organization_key() {
    let provider = FakeProvider::with(vec![tool_call_response(
        "submit",
        &json!({"actions": [{"title": "T", "description": "D", "quote": "Q"}]}),
        40,
        8,
    )]);
    let base = provider.serve().await;
    let target = target(
        ProviderKind::Mistral,
        "mistral-small-latest",
        Some(access(ProviderKind::Mistral, "mistral-key", &base)),
    );

    let extraction = runtime().extract(&target, "transcript").await.unwrap();

    assert_eq!(extraction.actions.len(), 1);
    assert_eq!(extraction.usage.model, "mistral-small-latest");
    assert_eq!(extraction.usage.input_tokens, 40);
    let request = &provider.recorded()[0];
    assert_eq!(request.path, "/v1/chat/completions");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer mistral-key")
    );
    assert_eq!(request.body["model"], "mistral-small-latest");
}

#[tokio::test]
async fn an_openai_compatible_endpoint_of_the_organization_replaces_the_worker_default() {
    let provider = FakeProvider::with(vec![tool_call_response(
        "submit",
        &json!({"same_task": false, "reason": "r"}),
        1,
        1,
    )]);
    let base = provider.serve().await;
    let target = target(
        ProviderKind::OpenAiCompatible,
        "llama3",
        Some(access(
            ProviderKind::OpenAiCompatible,
            "vllm-key",
            &format!("{base}/v1"),
        )),
    );
    let card = SimilarCard {
        card_id: CardId::new(),
        score: 0.7,
        text: "c".into(),
    };

    let judgement = runtime()
        .judge_duplicate(&target, &located("a", "q", 0, 1), &card)
        .await
        .unwrap();

    assert!(!judgement.same_task);
    let request = &provider.recorded()[0];
    assert_eq!(request.path, "/v1/chat/completions");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer vllm-key")
    );
}

fn embedding_reply(count: usize, width: usize) -> Value {
    let data: Vec<Value> = (0..count)
        .map(|index| json!({"object": "embedding", "index": index, "embedding": vec![0.5; width]}))
        .collect();
    json!({"object": "list", "data": data, "usage": {"prompt_tokens": 9, "total_tokens": 9}})
}

fn embedder() -> RigEmbedder {
    RigEmbedder::new(&args("http://127.0.0.1:1/v1", "http://127.0.0.1:1/mcp")).unwrap()
}

#[tokio::test]
async fn mistral_embeds_through_the_openai_shaped_endpoint() {
    let provider = FakeProvider::with(vec![embedding_reply(2, 1024)]);
    let base = provider.serve().await;
    let target = target(
        ProviderKind::Mistral,
        "mistral-embed",
        Some(access(ProviderKind::Mistral, "mistral-key", &base)),
    );

    let (embeddings, usage) = embedder()
        .embed(&target, &["un".to_owned(), "deux".to_owned()])
        .await
        .unwrap();

    assert_eq!(embeddings.len(), 2);
    assert_eq!(embeddings[0].0.len(), 1024);
    assert_eq!(usage.model, "mistral-embed");
    assert_eq!(usage.input_tokens, 9);
    let request = &provider.recorded()[0];
    assert_eq!(request.path, "/v1/embeddings");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer mistral-key")
    );
    assert_eq!(request.body["model"], "mistral-embed");
    assert_eq!(request.body["input"], json!(["un", "deux"]));
}

#[tokio::test]
async fn an_openai_compatible_endpoint_of_the_organization_embeds_with_any_dimension() {
    let provider = FakeProvider::with(vec![embedding_reply(1, 768)]);
    let base = provider.serve().await;
    let target = target(
        ProviderKind::OpenAiCompatible,
        "nomic-embed-text",
        Some(access(
            ProviderKind::OpenAiCompatible,
            "",
            &format!("{base}/v1"),
        )),
    );

    let (embeddings, _) = embedder().embed(&target, &["un".to_owned()]).await.unwrap();

    assert_eq!(embeddings[0].0.len(), 768);
    assert_eq!(provider.recorded()[0].path, "/v1/embeddings");
}

#[tokio::test]
async fn anthropic_has_no_embeddings() {
    let error = embedder()
        .embed(&claude("http://127.0.0.1:1"), &["un".to_owned()])
        .await
        .unwrap_err();

    assert!(
        matches!(&error, CoreError::Invalid(m) if m.contains("no embeddings")),
        "{error:?}"
    );
}

#[tokio::test]
async fn embeddings_of_inconsistent_sizes_are_refused() {
    let provider = FakeProvider::with(vec![json!({
        "data": [
            {"index": 0, "embedding": [1.0, 2.0]},
            {"index": 1, "embedding": [1.0]},
        ]
    })]);
    let base = provider.serve().await;
    let target = target(
        ProviderKind::Mistral,
        "mistral-embed",
        Some(access(ProviderKind::Mistral, "k", &base)),
    );

    let error = embedder()
        .embed(&target, &["a".to_owned(), "b".to_owned()])
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Internal(_)), "{error:?}");
}

fn catalog_access(kind: ProviderKind, secret: &str, base: &str) -> ProviderAccess {
    access(kind, secret, base)
}

#[tokio::test]
async fn the_anthropic_catalog_lists_chat_models_with_known_prices() {
    let provider = FakeProvider::with(vec![
        json!({
            "data": [
                {"type": "model", "id": "claude-sonnet-5-5", "display_name": "Claude Sonnet 5.5", "max_input_tokens": 1000000},
                {"type": "model", "id": "claude-haiku-5-5", "display_name": "Claude Haiku 5.5"},
            ],
            "has_more": true,
            "last_id": "claude-haiku-5-5",
        }),
        json!({
            "data": [{"type": "model", "id": "claude-legacy", "display_name": "Legacy"}],
            "has_more": false,
        }),
    ]);
    let base = provider.serve().await;

    let models = RigModelCatalog::new()
        .unwrap()
        .list_models(&catalog_access(ProviderKind::Anthropic, KEY, &base))
        .await
        .unwrap();

    let ids: Vec<_> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        ["claude-haiku-5-5", "claude-legacy", "claude-sonnet-5-5"]
    );
    let haiku = &models[0];
    assert_eq!(haiku.provider, ProviderKind::Anthropic);
    assert_eq!(haiku.display_name, "Claude Haiku 5.5");
    assert!(haiku.chat && !haiku.embeddings);
    assert_eq!(haiku.input_price_per_mtok_micros, Some(100_000));
    assert_eq!(haiku.output_price_per_mtok_micros, Some(500_000));
    assert_eq!(haiku.context_window, None);
    assert_eq!(models[2].context_window, Some(1_000_000));
    assert_eq!(models[2].input_price_per_mtok_micros, Some(2_000_000));
    assert_eq!(models[1].input_price_per_mtok_micros, None);
    let requests = provider.recorded();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, "GET");
    assert!(requests[0].path.starts_with("/v1/models?limit="));
    assert!(requests[1].path.contains("after_id=claude-haiku-5-5"));
    assert_claude_auth(&requests[0].headers);
}

#[tokio::test]
async fn the_mistral_catalog_tells_chat_from_embedding_models() {
    let provider = FakeProvider::with(vec![json!({
        "object": "list",
        "data": [
            {"id": "mistral-large-latest", "name": "mistral-large-latest", "max_context_length": 131072,
             "capabilities": {"completion_chat": true, "function_calling": true}},
            {"id": "mistral-embed", "name": "mistral-embed", "max_context_length": 8192,
             "capabilities": {"completion_chat": false}},
            {"id": "mistral-large-latest", "capabilities": {"completion_chat": true}},
            {"id": "mistral-moderation-latest", "capabilities": {"completion_chat": false}},
        ],
    })]);
    let base = provider.serve().await;

    let models = RigModelCatalog::new()
        .unwrap()
        .list_models(&catalog_access(ProviderKind::Mistral, "mistral-key", &base))
        .await
        .unwrap();

    let ids: Vec<_> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "mistral-embed",
            "mistral-large-latest",
            "mistral-moderation-latest"
        ]
    );
    assert!(models[0].embeddings && !models[0].chat);
    assert!(models[1].chat && !models[1].embeddings);
    assert_eq!(models[1].context_window, Some(131_072));
    assert_eq!(models[1].input_price_per_mtok_micros, None);
    assert!(!models[2].chat && !models[2].embeddings);
    let request = &provider.recorded()[0];
    assert_eq!(request.path, "/v1/models");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer mistral-key")
    );
}

#[tokio::test]
async fn the_openai_compatible_catalog_reads_models_under_the_base_url() {
    let provider = FakeProvider::with(vec![json!({
        "object": "list",
        "data": [
            {"id": "qwen2.5:7b-instruct", "object": "model"},
            {"id": "bge-m3", "object": "model"},
            {"id": "nomic-embed-text:latest", "object": "model"},
        ],
    })]);
    let base = provider.serve().await;

    let models = RigModelCatalog::new()
        .unwrap()
        .list_models(&catalog_access(
            ProviderKind::OpenAiCompatible,
            "",
            &format!("{base}/v1/"),
        ))
        .await
        .unwrap();

    let by_id = |id: &str| models.iter().find(|m| m.id == id).unwrap();
    assert!(by_id("qwen2.5:7b-instruct").chat && !by_id("qwen2.5:7b-instruct").embeddings);
    assert!(by_id("bge-m3").embeddings && !by_id("bge-m3").chat);
    assert!(by_id("nomic-embed-text:latest").embeddings);
    assert_eq!(by_id("bge-m3").provider, ProviderKind::OpenAiCompatible);
    assert_eq!(by_id("bge-m3").input_price_per_mtok_micros, None);
    let request = &provider.recorded()[0];
    assert_eq!(request.path, "/v1/models");
    assert!(!request.headers.contains_key("authorization"));
}

#[tokio::test]
async fn a_failing_provider_gives_a_readable_error_without_the_key() {
    let provider = FakeProvider::with_statuses(vec![(
        401,
        error_body("authentication_error", "invalid x-api-key"),
    )]);
    let base = provider.serve().await;

    let error = RigModelCatalog::new()
        .unwrap()
        .list_models(&catalog_access(ProviderKind::Anthropic, KEY, &base))
        .await
        .unwrap_err();

    let message = error.to_string();
    assert!(
        message.contains("401") && message.contains("invalid x-api-key"),
        "{message}"
    );
    assert!(!message.contains(KEY));
}

#[tokio::test]
async fn an_unreachable_provider_is_reported_not_panicked() {
    let error = RigModelCatalog::new()
        .unwrap()
        .list_models(&catalog_access(
            ProviderKind::Mistral,
            "k",
            "http://127.0.0.1:1",
        ))
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Internal(m) if m.contains("unreachable")));
}

#[tokio::test]
async fn voxtral_and_whisper_file_models_are_transcription_capable_and_nothing_else_is() {
    let mistral = FakeProvider::with(vec![json!({
        "data": [
            {"id": "voxtral-mini-latest", "capabilities": {"completion_chat": true}},
            {"id": "voxtral-mini-realtime-latest", "capabilities": {}},
            {"id": "voxtral-mini-tts-latest", "capabilities": {}},
            {"id": "mistral-large-latest", "capabilities": {"completion_chat": true}},
        ],
    })]);
    let mistral_base = mistral.serve().await;
    let local = FakeProvider::with(vec![json!({
        "data": [{"id": "Systran/faster-whisper-large-v3"}, {"id": "qwen2.5:7b-instruct"}],
    })]);
    let local_base = local.serve().await;
    let anthropic = FakeProvider::with(vec![json!({
        "data": [{"type": "model", "id": "claude-whisper-5"}],
        "has_more": false,
    })]);
    let anthropic_base = anthropic.serve().await;
    let catalog = RigModelCatalog::new().unwrap();

    let mistral_models = catalog
        .list_models(&catalog_access(ProviderKind::Mistral, "k", &mistral_base))
        .await
        .unwrap();
    let local_models = catalog
        .list_models(&catalog_access(
            ProviderKind::OpenAiCompatible,
            "",
            &format!("{local_base}/v1"),
        ))
        .await
        .unwrap();
    let anthropic_models = catalog
        .list_models(&catalog_access(
            ProviderKind::Anthropic,
            KEY,
            &anthropic_base,
        ))
        .await
        .unwrap();

    let capable = |models: &[capitainerie_core::domain::provider::ports::CatalogModel]| {
        models
            .iter()
            .filter(|model| model.transcription)
            .map(|model| model.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(capable(&mistral_models), ["voxtral-mini-latest"]);
    assert_eq!(capable(&local_models), ["Systran/faster-whisper-large-v3"]);
    assert!(capable(&anthropic_models).is_empty());
}

#[tokio::test]
async fn mistral_ocr_models_read_documents_and_serve_no_other_role() {
    let mistral = FakeProvider::with(vec![json!({
        "data": [
            {"id": "mistral-ocr-latest", "capabilities": {"completion_chat": true}},
            {"id": "mistral-ocr-2505", "capabilities": {}},
            {"id": "mistral-large-latest", "capabilities": {"completion_chat": true}},
            {"id": "voxtral-mini-latest", "capabilities": {}},
        ],
    })]);
    let mistral_base = mistral.serve().await;
    let local = FakeProvider::with(vec![json!({
        "data": [{"id": "some-ocr-model"}, {"id": "qwen2.5:7b-instruct"}],
    })]);
    let local_base = local.serve().await;
    let catalog = RigModelCatalog::new().unwrap();

    let mistral_models = catalog
        .list_models(&catalog_access(ProviderKind::Mistral, "k", &mistral_base))
        .await
        .unwrap();
    let local_models = catalog
        .list_models(&catalog_access(
            ProviderKind::OpenAiCompatible,
            "",
            &format!("{local_base}/v1"),
        ))
        .await
        .unwrap();

    let ocr: Vec<_> = mistral_models
        .iter()
        .filter(|model| model.ocr)
        .map(|model| model.id.as_str())
        .collect();
    assert_eq!(ocr, ["mistral-ocr-2505", "mistral-ocr-latest"]);
    for model in mistral_models.iter().filter(|model| model.ocr) {
        assert!(
            !model.chat && !model.embeddings && !model.transcription,
            "{}",
            model.id
        );
    }
    assert!(
        mistral_models
            .iter()
            .filter(|model| !model.ocr)
            .all(|model| model.chat || model.id == "voxtral-mini-latest")
    );
    assert!(local_models.iter().all(|model| !model.ocr));
}
