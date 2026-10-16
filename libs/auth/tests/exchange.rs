#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Form, Router};
use capitainerie_auth::{ClientCredentials, ExchangeArgs, FerrisKeyTokenExchange};
use capitainerie_common::CoreError;
use capitainerie_core::domain::agent::SecretToken;
use capitainerie_core::domain::delegation::ports::TokenExchanger;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

type Form_ = HashMap<String, String>;

struct Server {
    issuer: String,
    forms: Arc<Mutex<Vec<Form_>>>,
    discovery_hits: Arc<AtomicUsize>,
    deny: bool,
}

#[derive(Clone)]
struct AppState {
    issuer: String,
    forms: Arc<Mutex<Vec<Form_>>>,
    discovery_hits: Arc<AtomicUsize>,
    deny: bool,
}

async fn discovery(State(state): State<AppState>) -> axum::Json<Value> {
    state.discovery_hits.fetch_add(1, Ordering::SeqCst);
    axum::Json(json!({
        "issuer": state.issuer,
        "token_endpoint": format!("{}/protocol/openid-connect/token", state.issuer),
    }))
}

async fn token(
    State(state): State<AppState>,
    Form(form): Form<Form_>,
) -> (StatusCode, axum::Json<Value>) {
    let grant = form["grant_type"].clone();
    state.forms.lock().await.push(form);
    if state.deny {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": "unauthorized_client", "error_description": "not allowed"})),
        );
    }
    let access = if grant == "client_credentials" {
        "actor-token"
    } else {
        "issued-token"
    };
    (
        StatusCode::OK,
        axum::Json(json!({"access_token": access, "expires_in": 300, "token_type": "Bearer"})),
    )
}

async fn start(deny: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}/realms/acme", listener.local_addr().unwrap());
    let forms = Arc::new(Mutex::new(Vec::new()));
    let discovery_hits = Arc::new(AtomicUsize::new(0));
    let state = AppState {
        issuer: issuer.clone(),
        forms: forms.clone(),
        discovery_hits: discovery_hits.clone(),
        deny,
    };
    let app = Router::new()
        .route(
            "/realms/acme/.well-known/openid-configuration",
            get(discovery),
        )
        .route("/realms/acme/protocol/openid-connect/token", post(token))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Server {
        issuer,
        forms,
        discovery_hits,
        deny,
    }
}

fn creds(id: &str, secret: &str) -> ClientCredentials {
    ClientCredentials {
        client_id: id.to_owned(),
        client_secret: SecretToken::new(secret),
    }
}

fn exchanger() -> FerrisKeyTokenExchange {
    FerrisKeyTokenExchange::new(
        Some(creds("capitainerie-api", "api-secret")),
        vec![creds("agent-1", "agent-secret")],
        "capitainerie-mcp",
    )
}

#[tokio::test]
async fn for_agent_sends_the_exchange_form() {
    let server = start(false).await;
    let before = chrono::Utc::now();
    let out = exchanger()
        .for_agent(&server.issuer, &SecretToken::new("user-jwt"), "agent-1")
        .await
        .unwrap();
    assert_eq!(out.token.expose(), "issued-token");
    let delta = (out.expires_at - before).num_seconds();
    assert!((298..=302).contains(&delta), "{delta}");
    let forms = server.forms.lock().await;
    assert_eq!(forms.len(), 1);
    let f = &forms[0];
    assert_eq!(
        f["grant_type"],
        "urn:ietf:params:oauth:grant-type:token-exchange"
    );
    assert_eq!(f["client_id"], "capitainerie-api");
    assert_eq!(f["client_secret"], "api-secret");
    assert_eq!(f["subject_token"], "user-jwt");
    assert_eq!(
        f["subject_token_type"],
        "urn:ietf:params:oauth:token-type:access_token"
    );
    assert_eq!(f["audience"], "agent-1");
    assert!(!f.contains_key("actor_token"));
}

#[tokio::test]
async fn for_mcp_gets_actor_token_then_exchanges() {
    let server = start(false).await;
    let out = exchanger()
        .for_mcp(&server.issuer, &SecretToken::new("hop1"), "agent-1")
        .await
        .unwrap();
    assert_eq!(out.token.expose(), "issued-token");
    let forms = server.forms.lock().await;
    assert_eq!(forms.len(), 2);
    assert_eq!(forms[0]["grant_type"], "client_credentials");
    assert_eq!(forms[0]["client_id"], "agent-1");
    assert_eq!(forms[0]["client_secret"], "agent-secret");
    let f = &forms[1];
    assert_eq!(f["client_id"], "agent-1");
    assert_eq!(f["client_secret"], "agent-secret");
    assert_eq!(f["subject_token"], "hop1");
    assert_eq!(f["actor_token"], "actor-token");
    assert_eq!(
        f["actor_token_type"],
        "urn:ietf:params:oauth:token-type:access_token"
    );
    assert_eq!(f["audience"], "capitainerie-mcp");
}

#[tokio::test]
async fn oauth_error_maps_to_forbidden() {
    let server = start(true).await;
    assert!(server.deny);
    let err = exchanger()
        .for_agent(&server.issuer, &SecretToken::new("u"), "agent-1")
        .await
        .unwrap_err();
    match err {
        CoreError::Forbidden { reason } => {
            assert_eq!(reason.unwrap(), "unauthorized_client: not allowed");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn unknown_agent_makes_no_http_call() {
    let server = start(false).await;
    let err = exchanger()
        .for_mcp(&server.issuer, &SecretToken::new("t"), "ghost")
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Internal(_)));
    assert_eq!(server.discovery_hits.load(Ordering::SeqCst), 0);
    assert!(server.forms.lock().await.is_empty());
}

#[tokio::test]
async fn missing_requester_is_internal() {
    let server = start(false).await;
    let err = FerrisKeyTokenExchange::new(None, vec![], "capitainerie-mcp")
        .for_agent(&server.issuer, &SecretToken::new("u"), "agent-1")
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Internal(_)));
}

#[tokio::test]
async fn discovery_is_fetched_once() {
    let server = start(false).await;
    let ex = exchanger();
    for _ in 0..3 {
        ex.for_agent(&server.issuer, &SecretToken::new("u"), "agent-1")
            .await
            .unwrap();
    }
    ex.for_mcp(&server.issuer, &SecretToken::new("t"), "agent-1")
        .await
        .unwrap();
    assert_eq!(server.discovery_hits.load(Ordering::SeqCst), 1);
}

fn args(agents: &[&str]) -> ExchangeArgs {
    ExchangeArgs {
        client_id: Some("capitainerie-api".to_owned()),
        client_secret: Some("s".to_owned()),
        agent_credentials: agents.iter().map(|s| (*s).to_owned()).collect(),
        mcp_audience: "capitainerie-mcp".to_owned(),
    }
}

#[tokio::test]
async fn args_parse_agent_credentials() {
    let server = start(false).await;
    let ex = args(&["a=1", "b=2"]).exchanger().unwrap();
    ex.for_mcp(&server.issuer, &SecretToken::new("t"), "b")
        .await
        .unwrap();
    let forms = server.forms.lock().await;
    assert_eq!(forms[0]["client_id"], "b");
    assert_eq!(forms[0]["client_secret"], "2");
}

#[test]
fn args_reject_entry_without_equals() {
    assert!(args(&["a=1", "broken"]).exchanger().is_err());
}

#[test]
fn credentials_debug_hides_secret() {
    let shown = format!("{:?}", creds("agent-1", "topsecret"));
    assert!(!shown.contains("topsecret"));
}
