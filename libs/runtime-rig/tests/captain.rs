#![allow(clippy::unwrap_used)]

mod common;

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
    routing::post,
};
use capitainerie_core::domain::{
    agent::SecretToken,
    conversation::{
        ProposedCard,
        turn::{
            CaptainRuntime, ChatMessage, ChatRole, LocalTools, ToolSpec, TurnEnd, TurnEvent,
            TurnLimits, TurnRequest, TurnSink, TurnState,
        },
    },
    provider::ProviderKind,
};
use capitainerie_runtime_rig::RigCaptain;
use common::{access, args, local, target};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Extension, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
    },
};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use uuid::Uuid;

const CARD: Uuid = Uuid::from_u128(77);
const EXCHANGED: &str = "exchanged-captain-token";
const MEMBER: &str = "the-members-own-token";

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct CreateInput {
    title: String,
    description: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct SearchInput {
    query: Option<String>,
}

#[derive(Clone)]
struct Kanban {
    authorizations: Arc<Mutex<Vec<Option<String>>>>,
    created: Arc<Mutex<Vec<String>>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl Kanban {
    #[tool(description = "Create a card.")]
    async fn create_card(
        &self,
        Parameters(input): Parameters<CreateInput>,
        Extension(parts): Extension<axum::http::request::Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        self.authorizations.lock().unwrap().push(
            parts
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        );
        self.created.lock().unwrap().push(input.description);
        Ok(CallToolResult::success(vec![ContentBlock::text(
            json!({"id": CARD.to_string(), "title": input.title, "column": "proposed"}).to_string(),
        )]))
    }

    #[tool(description = "Search cards.")]
    async fn search_cards(
        &self,
        Parameters(input): Parameters<SearchInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let query = input.query.unwrap_or_default();
        Ok(CallToolResult::success(vec![ContentBlock::text(
            json!([{"id": "a", "title": format!("match {query}")}]).to_string(),
        )]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Kanban {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
}

async fn kanban() -> (Kanban, String) {
    let fake = Kanban {
        authorizations: Arc::default(),
        created: Arc::default(),
        tool_router: Kanban::tool_router(),
    };
    let handler = fake.clone();
    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Arc::new(NeverSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_sse_keep_alive(None)
            .disable_allowed_hosts(),
    );
    let app = Router::new().nest_service("/mcp", service);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (fake, url)
}

#[derive(Clone, Default)]
struct Model {
    bodies: Arc<Mutex<Vec<String>>>,
    headers: Arc<Mutex<Vec<HeaderMap>>>,
    replies: Arc<Mutex<Vec<(u16, String)>>>,
}

async fn model_reply(
    State(model): State<Model>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    model.bodies.lock().unwrap().push(body);
    model.headers.lock().unwrap().push(headers);
    let (status, text) = {
        let mut replies = model.replies.lock().unwrap();
        if replies.is_empty() {
            (200, String::new())
        } else {
            replies.remove(0)
        }
    };
    (
        StatusCode::from_u16(status).unwrap(),
        [(CONTENT_TYPE, "text/event-stream")],
        text,
    )
}

async fn serve_model(path: &'static str, replies: Vec<(u16, String)>) -> (Model, String) {
    let model = Model {
        replies: Arc::new(Mutex::new(replies)),
        ..Model::default()
    };
    let app = Router::new()
        .route(path, post(model_reply))
        .with_state(model.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (model, base)
}

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<String>()
        + "data: [DONE]\n\n"
}

fn anthropic_sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap()
            )
        })
        .collect()
}

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<TurnEvent>>,
}

impl TurnSink for Recorder {
    async fn emit(&self, event: TurnEvent) -> bool {
        self.events.lock().unwrap().push(event);
        true
    }
}

struct Repositories;

impl LocalTools for Repositories {
    fn definitions(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            name: "list_repositories".into(),
            description: "d".into(),
            parameters: json!({"type": "object"}),
        }]
    }

    async fn call(&self, _: &str, _: &Value) -> Result<String, String> {
        Ok("acme/site".into())
    }
}

fn request(target: capitainerie_core::domain::provider::ModelTarget) -> TurnRequest {
    TurnRequest {
        target,
        history: vec![ChatMessage {
            role: ChatRole::User,
            text: "Ajoute un README au dépôt test pour Acme".into(),
        }],
        mcp_token: SecretToken::new(EXCHANGED),
        traceparent: None,
        limits: TurnLimits::default(),
    }
}

#[tokio::test]
async fn an_openai_compatible_model_streams_calls_a_tool_and_proposes_a_card() {
    let (kanban, mcp_url) = kanban().await;
    let (model, base) = serve_model(
        "/v1/chat/completions",
        vec![
            (
                200,
                sse(&[
                    json!({"choices": [{"delta": {"content": "Je propose "}}]}),
                    json!({"choices": [{"delta": {"content": "une carte."}}]}),
                    json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "call_1", "type": "function", "function": {"name": "propose_card", "arguments": "{\"title\":\"Ajouter un README\","}}]}}]}),
                    json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "function": {"arguments": "\"description\":\"Pour Acme\"}"}}]}}]}),
                    json!({"choices": [], "usage": {"prompt_tokens": 100, "completion_tokens": 20}}),
                ]),
            ),
            (
                200,
                sse(&[
                    json!({"choices": [{"delta": {"content": "C'est prêt."}}]}),
                    json!({"choices": [], "usage": {"prompt_tokens": 130, "completion_tokens": 5}}),
                ]),
            ),
        ],
    )
    .await;
    let captain = RigCaptain::new(&args(&format!("{base}/v1"), &mcp_url)).unwrap();
    let sink = Recorder::default();
    let mut state = TurnState::default();

    let end = captain
        .run_turn(&request(local()), &Repositories, &sink, &mut state)
        .await
        .unwrap();

    assert_eq!(end, TurnEnd::Completed);
    let events = sink.events.lock().unwrap().clone();
    let shape: Vec<&str> = events
        .iter()
        .map(|event| match event {
            TurnEvent::Delta(_) => "delta",
            TurnEvent::ToolStarted { .. } => "tool_started",
            TurnEvent::ToolFinished { .. } => "tool_finished",
            TurnEvent::Card(_) => "card",
        })
        .collect();
    assert_eq!(
        shape,
        [
            "delta",
            "delta",
            "tool_started",
            "tool_finished",
            "card",
            "delta"
        ]
    );
    assert_eq!(
        state.cards,
        [ProposedCard {
            card_id: CARD,
            title: "Ajouter un README".into()
        }]
    );
    assert_eq!(state.text, "Je propose une carte.\n\nC'est prêt.");
    assert_eq!(state.tool_calls[0].name, "propose_card");
    assert!(state.tool_calls[0].succeeded);
    assert_eq!(
        (state.usage.input_tokens, state.usage.output_tokens),
        (230, 25)
    );
    assert_eq!(*kanban.created.lock().unwrap(), ["Pour Acme"]);
    assert_eq!(
        *kanban.authorizations.lock().unwrap(),
        [Some(format!("Bearer {EXCHANGED}"))]
    );
    let bodies = model.bodies.lock().unwrap().join("\n");
    assert!(!bodies.contains(MEMBER));
    assert!(!bodies.contains(EXCHANGED));
    assert!(bodies.contains("\"role\":\"tool\""));
    assert!(bodies.contains("\"name\":\"list_repositories\""));
}

#[tokio::test]
async fn an_anthropic_model_streams_text_and_tool_input_deltas() {
    let (kanban, mcp_url) = kanban().await;
    let (model, base) = serve_model(
        "/v1/messages",
        vec![
            (
                200,
                anthropic_sse(&[
                    json!({"type": "message_start", "message": {"usage": {"input_tokens": 60, "output_tokens": 1}}}),
                    json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
                    json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Je cherche."}}),
                    json!({"type": "content_block_stop", "index": 0}),
                    json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "toolu_1", "name": "search_cards", "input": {}}}),
                    json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"query\": \"read"}}),
                    json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "me\"}"}}),
                    json!({"type": "content_block_stop", "index": 1}),
                    json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 30}}),
                    json!({"type": "message_stop"}),
                ]),
            ),
            (
                200,
                anthropic_sse(&[
                    json!({"type": "message_start", "message": {"usage": {"input_tokens": 90, "output_tokens": 1}}}),
                    json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
                    json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Une carte existe."}}),
                    json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 8}}),
                    json!({"type": "message_stop"}),
                ]),
            ),
        ],
    )
    .await;
    let captain = RigCaptain::new(&args("http://127.0.0.1:1/v1", &mcp_url)).unwrap();
    let sink = Recorder::default();
    let mut state = TurnState::default();
    let target = target(
        ProviderKind::Anthropic,
        "claude-sonnet-5-5",
        Some(access(ProviderKind::Anthropic, "sk-ant-api03-key", &base)),
    );

    let end = captain
        .run_turn(&request(target), &Repositories, &sink, &mut state)
        .await
        .unwrap();

    assert_eq!(end, TurnEnd::Completed);
    assert_eq!(state.text, "Je cherche.\n\nUne carte existe.");
    assert_eq!(state.tool_calls[0].summary, "1 carte trouvée");
    assert_eq!(state.tool_calls[0].arguments, "{\"query\":\"readme\"}");
    assert!(state.cards.is_empty());
    assert_eq!(
        (state.usage.input_tokens, state.usage.output_tokens),
        (150, 38)
    );
    assert_eq!(state.usage.model, "claude-sonnet-5-5");
    assert_eq!(
        state.cost_micros,
        (150 * 2_000_000 + 38 * 10_000_000) / 1_000_000
    );
    assert!(kanban.created.lock().unwrap().is_empty());
    let headers = model.headers.lock().unwrap();
    assert_eq!(headers[0]["x-api-key"], "sk-ant-api03-key");
    let first: Value = serde_json::from_str(&model.bodies.lock().unwrap()[0]).unwrap();
    assert_eq!(first["stream"], true);
    assert_eq!(first["max_tokens"], 16_000);
    let names: Vec<_> = first["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "search_cards",
            "list_clients",
            "list_projects",
            "propose_card",
            "comment_card",
            "list_repositories"
        ]
    );
}

#[tokio::test]
async fn a_provider_failure_after_some_text_keeps_the_partial_answer() {
    let (_, mcp_url) = kanban().await;
    let (_, base) = serve_model(
        "/v1/chat/completions",
        vec![
            (
                200,
                "data: {\"choices\": [{\"delta\": {\"content\": \"Début\"}}]}\n\ndata: {\"error\": {\"message\": \"overloaded\"}}\n\n".to_owned(),
            ),
        ],
    )
    .await;
    let captain = RigCaptain::new(&args(&format!("{base}/v1"), &mcp_url)).unwrap();
    let sink = Recorder::default();
    let mut state = TurnState::default();

    let error = captain
        .run_turn(&request(local()), &Repositories, &sink, &mut state)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("overloaded"));
    assert_eq!(state.text, "Début");
}

#[tokio::test]
async fn an_http_error_from_the_provider_is_an_error_not_an_empty_answer() {
    let (_, mcp_url) = kanban().await;
    let (_, base) = serve_model(
        "/v1/chat/completions",
        vec![(401, r#"{"error":{"message":"bad key"}}"#.to_owned())],
    )
    .await;
    let captain = RigCaptain::new(&args(&format!("{base}/v1"), &mcp_url)).unwrap();
    let mut state = TurnState::default();

    let error = captain
        .run_turn(
            &request(local()),
            &Repositories,
            &Recorder::default(),
            &mut state,
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("bad key"), "{error}");
    assert!(state.text.is_empty());
}
