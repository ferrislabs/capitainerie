#![allow(dead_code, clippy::unwrap_used)]

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

#[derive(Clone, Default)]
pub struct FakeProvider {
    replies: Arc<Mutex<VecDeque<(u16, Value)>>>,
    pub requests: Arc<Mutex<Vec<Recorded>>>,
}

impl FakeProvider {
    pub fn with(replies: Vec<Value>) -> Self {
        Self::with_statuses(replies.into_iter().map(|reply| (200, reply)).collect())
    }

    pub fn with_statuses(replies: Vec<(u16, Value)>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into())),
            requests: Arc::default(),
        }
    }

    pub async fn serve(&self) -> String {
        let app = Router::new().fallback(handle).with_state(self.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        base
    }

    pub fn recorded(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

async fn handle(
    State(provider): State<FakeProvider>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    provider.requests.lock().unwrap().push(Recorded {
        method: method.to_string(),
        path: uri
            .path_and_query()
            .map(ToString::to_string)
            .unwrap_or_default(),
        headers: headers
            .iter()
            .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_owned())))
            .collect(),
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
    });
    let next = provider.replies.lock().unwrap().pop_front();
    match next {
        Some((status, reply)) => {
            (StatusCode::from_u16(status).unwrap(), axum::Json(reply)).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn message(content: Value, stop_reason: &str, input: u64, output: u64) -> Value {
    json!({
        "id": "msg_1",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-5-5",
        "content": content,
        "stop_reason": stop_reason,
        "usage": { "input_tokens": input, "output_tokens": output },
    })
}

pub fn text_message(text: &str, input: u64, output: u64) -> Value {
    message(
        json!([{ "type": "text", "text": text }]),
        "end_turn",
        input,
        output,
    )
}

pub fn tool_use(id: &str, name: &str, input: &Value) -> Value {
    json!({ "type": "tool_use", "id": id, "name": name, "input": input })
}

pub fn refusal(category: &str) -> Value {
    json!({
        "id": "msg_2",
        "type": "message",
        "role": "assistant",
        "content": [],
        "stop_reason": "refusal",
        "stop_details": { "type": "refusal", "category": category, "explanation": "declined" },
        "usage": { "input_tokens": 9, "output_tokens": 0 },
    })
}

pub fn error_body(kind: &str, message: &str) -> Value {
    json!({ "type": "error", "error": { "type": kind, "message": message } })
}
