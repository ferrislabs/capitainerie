#![allow(dead_code, clippy::unwrap_used)]

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{Json, Router, extract::State, http::request::Parts, routing::post};
use capitainerie_core::domain::{
    agent::SecretToken,
    provider::{ModelRef, ModelTarget, ProviderAccess, ProviderKind},
};
use capitainerie_runtime_rig::LlmArgs;
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

pub mod providers;

pub const MODEL: &str = "fake-model";

#[derive(Clone, Default)]
pub struct FakeLlm {
    responses: Arc<Mutex<VecDeque<Value>>>,
    pub requests: Arc<Mutex<Vec<Value>>>,
    pub embedding_requests: Arc<Mutex<Vec<Value>>>,
}

impl FakeLlm {
    pub fn with(responses: Vec<Value>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into())),
            requests: Arc::default(),
            embedding_requests: Arc::default(),
        }
    }

    pub fn request_bodies(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(Value::to_string)
            .collect()
    }

    pub async fn serve(&self) -> String {
        let app = Router::new()
            .route("/v1/chat/completions", post(completions))
            .route("/v1/embeddings", post(embeddings))
            .with_state(self.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        base
    }
}

async fn completions(State(llm): State<FakeLlm>, Json(body): Json<Value>) -> Json<Value> {
    llm.requests.lock().unwrap().push(body);
    let next = llm.responses.lock().unwrap().pop_front();
    Json(next.unwrap_or_else(|| text_response("done", 0, 0)))
}

async fn embeddings(State(llm): State<FakeLlm>, Json(body): Json<Value>) -> Json<Value> {
    let count = body["input"].as_array().map_or(0, Vec::len);
    llm.embedding_requests.lock().unwrap().push(body);
    let data: Vec<Value> = (0..count)
        .rev()
        .map(|index| json!({"object": "embedding", "index": index, "embedding": [index as f32, 1.0, 0.0, 0.5]}))
        .collect();
    Json(json!({
        "object": "list",
        "model": MODEL,
        "data": data,
        "usage": {"prompt_tokens": 7 * count, "total_tokens": 7 * count},
    }))
}

fn envelope(message: Value, finish: &str, input: u64, output: u64) -> Value {
    json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "created": 0,
        "model": MODEL,
        "choices": [{"index": 0, "message": message, "finish_reason": finish}],
        "usage": {
            "prompt_tokens": input,
            "completion_tokens": output,
            "total_tokens": input + output,
        },
    })
}

pub fn text_response(text: &str, input: u64, output: u64) -> Value {
    envelope(
        json!({"role": "assistant", "content": text}),
        "stop",
        input,
        output,
    )
}

pub fn tool_call_response(name: &str, arguments: &Value, input: u64, output: u64) -> Value {
    envelope(
        json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {"name": name, "arguments": arguments.to_string()},
            }],
        }),
        "tool_calls",
        input,
        output,
    )
}

#[derive(Debug, Clone)]
pub struct RecordedCall {
    pub arguments: Value,
    pub authorization: Option<String>,
    pub traceparent: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreateCardInput {
    pub title: String,
    pub description: String,
    pub run_id: Option<String>,
    pub provenance: Option<Value>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub priority: Option<String>,
    pub estimate: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CommentInput {
    pub card_id: String,
    pub body: String,
    pub run_id: Option<String>,
}

#[derive(Clone)]
pub struct FakeMcp {
    pub calls: Arc<Mutex<Vec<RecordedCall>>>,
    pub comments: Arc<Mutex<Vec<RecordedCall>>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl FakeMcp {
    #[tool(description = "Create a card.")]
    async fn create_card(
        &self,
        Parameters(input): Parameters<CreateCardInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let header = |name: &str| {
            parts
                .headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        self.calls.lock().unwrap().push(RecordedCall {
            arguments: json!({
                "title": input.title,
                "description": input.description,
                "run_id": input.run_id,
                "provenance": input.provenance,
                "acceptance_criteria": input.acceptance_criteria,
                "priority": input.priority,
                "estimate": input.estimate,
                "due_date": input.due_date,
            }),
            authorization: header("authorization"),
            traceparent: header("traceparent"),
        });
        if input.title.contains("REFUSE") {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "forbidden: the quote does not match the transcript",
            )]));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            json!({"id": "card-1", "title": input.title}).to_string(),
        )]))
    }

    #[tool(description = "Comment on a card.")]
    async fn comment(
        &self,
        Parameters(input): Parameters<CommentInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let header = |name: &str| {
            parts
                .headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        self.comments.lock().unwrap().push(RecordedCall {
            arguments: json!({
                "card_id": input.card_id,
                "body": input.body,
                "run_id": input.run_id,
            }),
            authorization: header("authorization"),
            traceparent: header("traceparent"),
        });
        if input.body.contains("REFUSE") {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "forbidden: cannot comment",
            )]));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            json!({"id": "comment-1", "card_id": input.card_id, "body": input.body}).to_string(),
        )]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for FakeMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
}

impl FakeMcp {
    pub async fn serve() -> (Self, String) {
        let fake = Self {
            calls: Arc::default(),
            comments: Arc::default(),
            tool_router: Self::tool_router(),
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

    pub fn recorded(&self) -> Vec<RecordedCall> {
        self.calls.lock().unwrap().clone()
    }

    pub fn recorded_comments(&self) -> Vec<RecordedCall> {
        self.comments.lock().unwrap().clone()
    }
}

pub fn args(base_url: &str, mcp_url: &str) -> LlmArgs {
    LlmArgs {
        base_url: base_url.to_owned(),
        api_key: "test-key".to_owned(),
        model: MODEL.to_owned(),
        timeout_secs: 30,
        mcp_url: mcp_url.to_owned(),
    }
}

pub fn target(kind: ProviderKind, model: &str, access: Option<ProviderAccess>) -> ModelTarget {
    ModelTarget {
        model: ModelRef {
            provider: kind,
            model: model.to_owned(),
        },
        access,
    }
}

pub fn local() -> ModelTarget {
    target(ProviderKind::OpenAiCompatible, MODEL, None)
}

pub fn access(kind: ProviderKind, secret: &str, base_url: &str) -> ProviderAccess {
    ProviderAccess {
        kind,
        secret: SecretToken::new(secret),
        base_url: Some(base_url.to_owned()),
    }
}
