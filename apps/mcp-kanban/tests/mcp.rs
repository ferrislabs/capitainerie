#![allow(clippy::unwrap_used)]

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{Router, extract::State, routing::get};
use capitainerie_auth::{AcceptedClients, FerrisKeyVerifier};
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{authorization::Subject, run::service::SubmitTranscript},
    infrastructure::postgres,
};
use capitainerie_mcp_kanban::router;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, CallToolResult, ClientConfig},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const KEY_A_PEM: &str = include_str!("../../../libs/auth/tests/fixtures/test-only-key-a.pem");
const KEY_A_JWK: &str = include_str!("../../../libs/auth/tests/fixtures/test-only-key-a.jwk.json");
const AGENT: &str = "capitainerie-agent-writer";
const AUDIENCE: &str = "capitainerie-mcp";

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

#[derive(Clone)]
struct IdpState {
    issuer: String,
    keys: Value,
}

async fn discovery(State(state): State<IdpState>) -> axum::Json<Value> {
    axum::Json(json!({
        "issuer": state.issuer,
        "jwks_uri": format!("{}/protocol/openid-connect/jwks.json", state.issuer),
    }))
}

async fn jwks(State(state): State<IdpState>) -> axum::Json<Value> {
    axum::Json(state.keys)
}

struct Harness {
    usecase: Option<CapitainerieUseCase>,
    organization_id: Uuid,
    issuer: String,
    base: String,
    http: reqwest::Client,
}

async fn identity_provider(realm: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}/realms/{realm}", listener.local_addr().unwrap());
    let mut key: Value = serde_json::from_str(KEY_A_JWK).unwrap();
    key["kid"] = json!("kid-a");
    let state = IdpState {
        issuer: issuer.clone(),
        keys: json!({ "keys": [key] }),
    };
    let app = Router::new()
        .route(
            &format!("/realms/{realm}/.well-known/openid-configuration"),
            get(discovery),
        )
        .route(
            &format!("/realms/{realm}/protocol/openid-connect/jwks.json"),
            get(jwks),
        )
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    issuer
}

async fn harness() -> Option<Harness> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool: PgPool = postgres::connect(&url, 4).await.unwrap();
    let id = Uuid::now_v7();
    let realm = format!("org-{}", id.simple());
    let issuer = identity_provider(&realm).await;
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(realm)
    .bind(&issuer)
    .execute(&pool)
    .await
    .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let verifier = FerrisKeyVerifier::with_audience(AcceptedClients::new([AGENT]), AUDIENCE);
    let usecase = CapitainerieUseCase::with_local_policies(pool);
    let app = router(usecase.clone(), verifier, base.clone());
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Some(Harness {
        usecase: Some(usecase),
        organization_id: id,
        issuer,
        base,
        http: reqwest::Client::new(),
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

impl Harness {
    fn token(&self, audience: &str, delegated: bool) -> String {
        let mut claims = json!({
            "sub": "7b0f0c1e-0000-4000-8000-000000000001",
            "iss": self.issuer,
            "typ": "Bearer",
            "azp": AGENT,
            "aud": [audience],
            "exp": now() + 300,
            "iat": now(),
        });
        if delegated {
            claims["act"] = json!({"client_id": AGENT, "sub": "service-account-agent-writer"});
        }
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("kid-a".to_owned());
        let key = EncodingKey::from_rsa_pem(KEY_A_PEM.as_bytes()).unwrap();
        jsonwebtoken::encode(&header, &claims, &key).unwrap()
    }

    async fn transcript(&self, body: &str) -> String {
        let usecase = self.usecase.as_ref().unwrap();
        let submitted = usecase
            .submit_transcript(SubmitTranscript {
                subject: Subject {
                    id: "7b0f0c1e-0000-4000-8000-000000000001".into(),
                    organization_id: OrganizationId(self.organization_id),
                    actor: None,
                    is_admin: false,
                },
                title: "Weekly".into(),
                body: body.into(),
            })
            .await
            .unwrap();
        submitted.transcript_id.0.to_string()
    }

    async fn post_mcp(&self, token: Option<&str>) -> reqwest::Response {
        let mut request = self
            .http
            .post(format!("{}/mcp", self.base))
            .header("accept", "application/json, text/event-stream")
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}));
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        request.send().await.unwrap()
    }

    async fn client(
        &self,
        token: String,
    ) -> rmcp::service::RunningService<rmcp::RoleClient, ClientConfig> {
        let transport = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(format!("{}/mcp", self.base))
                .auth_header(token),
        );
        ClientConfig::default().serve(transport).await.unwrap()
    }
}

fn call(name: &'static str, arguments: Value) -> CallToolRequestParams {
    CallToolRequestParams::new(name).with_arguments(arguments.as_object().cloned().unwrap())
}

fn payload(result: &CallToolResult) -> Value {
    let value = serde_json::to_value(result).unwrap();
    let text = value["content"][0]["text"].as_str().unwrap_or_default();
    serde_json::from_str(text).unwrap_or(Value::String(text.to_owned()))
}

#[tokio::test]
async fn health_and_protected_resource_metadata_need_no_token() {
    let Some(h) = harness().await else { return };
    let live = h
        .http
        .get(format!("{}/health/live", h.base))
        .send()
        .await
        .unwrap();
    assert_eq!(live.status(), 204);
    let metadata: Value = h
        .http
        .get(format!("{}/.well-known/oauth-protected-resource", h.base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(metadata["resource"], h.base);
    assert_eq!(metadata["bearer_methods_supported"], json!(["header"]));
}

#[tokio::test]
async fn a_request_without_a_token_is_challenged() {
    let Some(h) = harness().await else { return };
    let response = h.post_mcp(None).await;
    assert_eq!(response.status(), 401);
    let challenge = response.headers()["www-authenticate"].to_str().unwrap();
    assert!(challenge.starts_with("Bearer"), "{challenge}");
    assert!(
        challenge.contains(&format!(
            r#"resource_metadata="{}/.well-known/oauth-protected-resource""#,
            h.base
        )),
        "{challenge}"
    );
}

#[tokio::test]
async fn a_token_for_another_audience_is_refused() {
    let Some(h) = harness().await else { return };
    let response = h.post_mcp(Some(&h.token("capitainerie-api", true))).await;
    assert_eq!(response.status(), 401);
    assert!(
        response.headers()["www-authenticate"]
            .to_str()
            .unwrap()
            .contains("resource_metadata")
    );
}

#[tokio::test]
async fn a_token_from_an_unknown_issuer_is_refused() {
    let Some(h) = harness().await else { return };
    let mut other = harness().await.unwrap();
    other.base = h.base.clone();
    let response = other.post_mcp(Some(&other.token(AUDIENCE, true))).await;
    assert_eq!(response.status(), 200);
    let stranger = Harness {
        usecase: None,
        organization_id: Uuid::nil(),
        issuer: "http://127.0.0.1:1/realms/nobody".into(),
        base: h.base.clone(),
        http: reqwest::Client::new(),
    };
    let response = stranger
        .post_mcp(Some(&stranger.token(AUDIENCE, true)))
        .await;
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn a_token_that_is_not_delegated_is_forbidden() {
    let Some(h) = harness().await else { return };
    let response = h.post_mcp(Some(&h.token(AUDIENCE, false))).await;
    assert_eq!(response.status(), 403);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body, json!({"error": "delegation_required"}));
}

#[tokio::test]
async fn a_delegated_token_lists_the_four_tools() {
    let Some(h) = harness().await else { return };
    let client = h.client(h.token(AUDIENCE, true)).await;
    let mut names: Vec<String> = client
        .list_all_tools()
        .await
        .unwrap()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["comment", "create_card", "move_card", "update_card"]
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn an_agent_proposes_a_card_and_cannot_move_it_out_of_proposed() {
    let Some(h) = harness().await else { return };
    let transcript = h.transcript("Elodie : je relance Acme demain.").await;
    let client = h.client(h.token(AUDIENCE, true)).await;

    let missing = client
        .call_tool(call(
            "create_card",
            json!({"title": "Sans source", "description": "x"}),
        ))
        .await
        .unwrap();
    assert_eq!(missing.is_error, Some(true), "{missing:?}");

    let created = client
        .call_tool(call(
            "create_card",
            json!({
                "title": "Relancer Acme",
                "description": "Avant vendredi",
                "provenance": {
                    "transcript_id": transcript,
                    "start": 9,
                    "end": 31,
                    "quote": "je relance Acme demain"
                }
            }),
        ))
        .await
        .unwrap();
    assert_ne!(created.is_error, Some(true), "{created:?}");
    let card = payload(&created);
    assert_eq!(card["column"], "proposed");
    assert_eq!(card["title"], "Relancer Acme");
    let id = card["id"].as_str().unwrap().to_owned();
    let version = card["version"].as_i64().unwrap();

    let moved = client
        .call_tool(call(
            "move_card",
            json!({"card_id": id, "to": "todo", "expected_version": version}),
        ))
        .await
        .unwrap();
    assert_eq!(moved.is_error, Some(true), "{moved:?}");
    assert!(
        payload(&moved)
            .as_str()
            .unwrap_or_default()
            .contains("forbidden"),
        "{moved:?}"
    );

    let commented = client
        .call_tool(call("comment", json!({"card_id": id, "body": "Urgent"})))
        .await
        .unwrap();
    assert_ne!(commented.is_error, Some(true), "{commented:?}");

    let invalid = client
        .call_tool(call("comment", json!({"card_id": "nope", "body": "x"})))
        .await
        .unwrap();
    assert_eq!(invalid.is_error, Some(true));
    client.cancel().await.unwrap();
}
