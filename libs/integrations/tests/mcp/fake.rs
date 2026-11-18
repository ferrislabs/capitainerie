#![allow(dead_code, clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use capitainerie_core::domain::mcp::pkce::challenge_of;
use serde_json::{Value, json};
use tokio::net::TcpListener;

pub const GOOD_CODE: &str = "the-code";

#[derive(Clone, Copy, Default)]
pub struct Options {
    pub hint_in_challenge: bool,
    pub openid_only: bool,
    pub no_resource_metadata: bool,
}

#[derive(Debug, Clone)]
pub struct Seen {
    pub method: String,
    pub path: String,
    pub authorization: Option<String>,
    pub body: String,
}

impl Seen {
    pub fn form(&self) -> HashMap<String, String> {
        url::form_urlencoded::parse(self.body.as_bytes())
            .into_owned()
            .collect()
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

pub struct FakeState {
    pub base: String,
    options: Options,
    pub seen: Mutex<Vec<Seen>>,
    pub lifetime: AtomicI64,
    issued: AtomicUsize,
    current_access: Mutex<Option<String>>,
    current_refresh: Mutex<Option<String>>,
    challenges: Mutex<HashMap<String, String>>,
}

pub struct Fake {
    pub base: String,
    pub state: Arc<FakeState>,
}

impl Fake {
    pub fn requests(&self, method: &str, path: &str) -> Vec<Seen> {
        self.state
            .seen
            .lock()
            .unwrap()
            .iter()
            .filter(|seen| seen.method == method && seen.path == path)
            .cloned()
            .collect()
    }

    pub fn total_requests(&self) -> usize {
        self.state.seen.lock().unwrap().len()
    }

    pub fn expire_access_token(&self) {
        *self.state.current_access.lock().unwrap() = None;
    }

    pub fn set_lifetime(&self, seconds: i64) {
        self.state.lifetime.store(seconds, Ordering::SeqCst);
    }

    pub fn issued_tokens(&self) -> usize {
        self.state.issued.load(Ordering::SeqCst)
    }
}

pub async fn start(options: Options) -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let state = Arc::new(FakeState {
        base: base.clone(),
        options,
        seen: Mutex::default(),
        lifetime: AtomicI64::new(3600),
        issued: AtomicUsize::new(0),
        current_access: Mutex::default(),
        current_refresh: Mutex::default(),
        challenges: Mutex::default(),
    });
    let app = Router::new()
        .fallback(handle)
        .with_state(Arc::clone(&state));
    tokio::spawn(async move { axum::serve(listener, app).await });
    Fake { base, state }
}

fn json_response(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn text_of(length: usize) -> String {
    "é".repeat(length)
}

async fn handle(State(state): State<Arc<FakeState>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let bytes: Bytes = axum::body::to_bytes(body, 1 << 20).await.unwrap();
    let seen = Seen {
        method: parts.method.to_string(),
        path: parts.uri.path().to_owned(),
        authorization: parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        body: String::from_utf8_lossy(&bytes).into_owned(),
    };
    state.seen.lock().unwrap().push(seen.clone());
    let base = &state.base;
    match (seen.method.as_str(), seen.path.as_str()) {
        ("POST", "/mcp") => mcp(&state, &parts.headers, &seen),
        ("GET", "/mcp") => StatusCode::METHOD_NOT_ALLOWED.into_response(),
        ("GET", "/.well-known/oauth-protected-resource") if !state.options.no_resource_metadata => {
            json_response(
                StatusCode::OK,
                &json!({
                    "resource": format!("{base}/mcp"),
                    "authorization_servers": [format!("{base}/realms/mestier")],
                    "scopes_supported": ["mestier.read", "mestier.doe.write"],
                }),
            )
        }
        ("GET", "/realms/mestier/.well-known/oauth-authorization-server")
            if !state.options.openid_only =>
        {
            json_response(StatusCode::OK, &server_metadata(base))
        }
        ("GET", "/realms/mestier/.well-known/openid-configuration") => {
            json_response(StatusCode::OK, &server_metadata(base))
        }
        ("POST", "/realms/mestier/clients/register") => json_response(
            StatusCode::CREATED,
            &json!({"client_id": "client-xyz", "client_secret": "must-be-ignored"}),
        ),
        ("GET", "/realms/mestier/auth") => authorize(&state, &parts.uri),
        ("POST", "/realms/mestier/token") => token(&state, &seen),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

fn server_metadata(base: &str) -> Value {
    json!({
        "issuer": format!("{base}/realms/mestier"),
        "authorization_endpoint": format!("{base}/realms/mestier/auth"),
        "token_endpoint": format!("{base}/realms/mestier/token"),
        "registration_endpoint": format!("{base}/realms/mestier/clients/register"),
        "code_challenge_methods_supported": ["S256"],
    })
}

fn mcp(state: &FakeState, headers: &HeaderMap, seen: &Seen) -> Response {
    let expected = state.current_access.lock().unwrap().clone();
    let presented = seen
        .authorization
        .as_deref()
        .and_then(|value| value.strip_prefix("Bearer "));
    if expected.is_none() || presented != expected.as_deref() {
        let challenge = if state.options.hint_in_challenge {
            format!(
                "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
                state.base
            )
        } else {
            "Bearer".to_owned()
        };
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, challenge)],
        )
            .into_response();
    }
    let _ = headers;
    let message = seen.json();
    let id = message["id"].clone();
    match message["method"].as_str().unwrap_or_default() {
        "initialize" => json_response(
            StatusCode::OK,
            &json!({"jsonrpc": "2.0", "id": id, "result": {
                "protocolVersion": message["params"]["protocolVersion"],
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "fake-mestier", "version": "1"},
            }}),
        ),
        "tools/list" => json_response(
            StatusCode::OK,
            &json!({"jsonrpc": "2.0", "id": id, "result": {"tools": [
                {"name": "list_doe", "description": "Lister les DOE",
                 "inputSchema": {"type": "object", "properties": {}},
                 "annotations": {"readOnlyHint": true}},
                {"name": "write_doe", "inputSchema": {"type": "object"},
                 "annotations": {"readOnlyHint": false, "destructiveHint": true}},
                {"name": "bare", "inputSchema": {"type": "object"}},
            ]}}),
        ),
        "tools/call" => {
            let name = message["params"]["name"].as_str().unwrap_or_default();
            let (text, is_error) = match name {
                "big" => (text_of(20_000), false),
                "boom" => ("échec".to_owned(), true),
                other => (
                    format!("called {other} {}", message["params"]["arguments"]),
                    false,
                ),
            };
            json_response(
                StatusCode::OK,
                &json!({"jsonrpc": "2.0", "id": id, "result": {
                    "content": [{"type": "text", "text": text}], "isError": is_error}}),
            )
        }
        _ => StatusCode::ACCEPTED.into_response(),
    }
}

fn authorize(state: &FakeState, uri: &axum::http::Uri) -> Response {
    let query: HashMap<String, String> =
        url::form_urlencoded::parse(uri.query().unwrap_or("").as_bytes())
            .into_owned()
            .collect();
    let valid = query.get("response_type").map(String::as_str) == Some("code")
        && query.get("client_id").map(String::as_str) == Some("client-xyz")
        && query.get("code_challenge_method").map(String::as_str) == Some("S256")
        && query.get("resource").map(String::as_str) == Some(&format!("{}/mcp", state.base));
    let (Some(challenge), Some(redirect), Some(echo)) = (
        query.get("code_challenge"),
        query.get("redirect_uri"),
        query.get("state"),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if !valid {
        return StatusCode::BAD_REQUEST.into_response();
    }
    state
        .challenges
        .lock()
        .unwrap()
        .insert(GOOD_CODE.to_owned(), challenge.clone());
    (
        StatusCode::FOUND,
        [(
            header::LOCATION,
            format!("{redirect}?code={GOOD_CODE}&state={echo}"),
        )],
    )
        .into_response()
}

fn token(state: &FakeState, seen: &Seen) -> Response {
    let form = seen.form();
    let refused = |error: &str| {
        json_response(
            StatusCode::BAD_REQUEST,
            &json!({"error": error, "error_description": "do not leak"}),
        )
    };
    if form.get("resource").map(String::as_str) != Some(&format!("{}/mcp", state.base))
        || form.get("client_id").map(String::as_str) != Some("client-xyz")
        || form.contains_key("client_secret")
    {
        return refused("invalid_request");
    }
    match form.get("grant_type").map(String::as_str) {
        Some("authorization_code") => {
            let challenge = state
                .challenges
                .lock()
                .unwrap()
                .remove(form.get("code").map(String::as_str).unwrap_or_default());
            let verifier = form
                .get("code_verifier")
                .map(String::as_str)
                .unwrap_or_default();
            if challenge.as_deref() != Some(challenge_of(verifier).as_str()) {
                return refused("invalid_grant");
            }
        }
        Some("refresh_token") => {
            let current = state.current_refresh.lock().unwrap().clone();
            if form.get("refresh_token") != current.as_ref() {
                return refused("invalid_grant");
            }
        }
        _ => return refused("unsupported_grant_type"),
    }
    let n = state.issued.fetch_add(1, Ordering::SeqCst) + 1;
    let (access, refresh) = (format!("at-{n}"), format!("rt-{n}"));
    *state.current_access.lock().unwrap() = Some(access.clone());
    *state.current_refresh.lock().unwrap() = Some(refresh.clone());
    json_response(
        StatusCode::OK,
        &json!({
            "access_token": access,
            "token_type": "Bearer",
            "expires_in": state.lifetime.load(Ordering::SeqCst),
            "refresh_token": refresh,
            "scope": "mestier.read mestier.doe.write",
        }),
    )
}
