#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::State;
use axum::routing::get;
use capitainerie_auth::{AcceptedClients, AuthError, FerrisKeyVerifier, TokenVerifier};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

const KEY_A_PEM: &str = include_str!("fixtures/test-only-key-a.pem");
const KEY_B_PEM: &str = include_str!("fixtures/test-only-key-b.pem");
const KEY_A_JWK: &str = include_str!("fixtures/test-only-key-a.jwk.json");
const KEY_B_JWK: &str = include_str!("fixtures/test-only-key-b.jwk.json");

struct Server {
    issuer: String,
    jwks_hits: Arc<AtomicUsize>,
    served: Arc<Mutex<Vec<Value>>>,
}

#[derive(Clone)]
struct AppState {
    issuer: String,
    jwks_hits: Arc<AtomicUsize>,
    served: Arc<Mutex<Vec<Value>>>,
}

fn jwk(raw: &str, kid: &str) -> Value {
    let mut value: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    value["kid"] = json!(kid);
    value
}

async fn discovery(State(state): State<AppState>) -> axum::Json<Value> {
    axum::Json(json!({
        "issuer": state.issuer,
        "jwks_uri": format!("{}/protocol/openid-connect/jwks.json", state.issuer),
    }))
}

async fn jwks(State(state): State<AppState>) -> axum::Json<Value> {
    state.jwks_hits.fetch_add(1, Ordering::SeqCst);
    let keys = state.served.lock().await.clone();
    axum::Json(json!({ "keys": keys }))
}

async fn start() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}/realms/acme", listener.local_addr().unwrap());
    let jwks_hits = Arc::new(AtomicUsize::new(0));
    let served = Arc::new(Mutex::new(vec![
        json!({"kty": "EC", "kid": "ec-skipped", "crv": "P-256", "x": "AA", "y": "AA"}),
        jwk(KEY_A_JWK, "kid-a"),
    ]));
    let state = AppState {
        issuer: issuer.clone(),
        jwks_hits: jwks_hits.clone(),
        served: served.clone(),
    };
    let app = Router::new()
        .route(
            "/realms/acme/.well-known/openid-configuration",
            get(discovery),
        )
        .route("/realms/acme/protocol/openid-connect/jwks.json", get(jwks))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Server {
        issuer,
        jwks_hits,
        served,
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

fn claims(issuer: &str) -> Value {
    json!({
        "sub": "7b0f0c1e-0000-4000-8000-000000000001",
        "iss": issuer,
        "typ": "Bearer",
        "azp": "capitainerie-web",
        "aud": ["acme-realm", "account"],
        "exp": now() + 300,
        "iat": now(),
        "preferred_username": "jdoe",
        "email": "jdoe@example.com",
        "given_name": "Jane",
        "family_name": "Doe",
    })
}

fn sign(claims: &Value, pem: &str, kid: &str) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_owned());
    let key = EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();
    jsonwebtoken::encode(&header, claims, &key).unwrap()
}

fn verifier() -> FerrisKeyVerifier {
    FerrisKeyVerifier::with_refresh_interval(
        AcceptedClients::new(["capitainerie-web", "capitainerie-agent"]),
        Duration::ZERO,
    )
}

#[tokio::test]
async fn valid_token_yields_identity() {
    let server = start().await;
    let token = sign(&claims(&server.issuer), KEY_A_PEM, "kid-a");
    let identity = verifier().verify(&token, &server.issuer).await.unwrap();
    assert_eq!(identity.subject, "7b0f0c1e-0000-4000-8000-000000000001");
    assert_eq!(identity.issuer, server.issuer);
    assert_eq!(identity.client_id, "capitainerie-web");
    assert_eq!(identity.username.as_deref(), Some("jdoe"));
    assert_eq!(identity.email.as_deref(), Some("jdoe@example.com"));
    assert_eq!(identity.display_name.as_deref(), Some("Jane Doe"));
    assert_eq!(identity.actor, None);
}

#[tokio::test]
async fn expired_token_is_rejected() {
    let server = start().await;
    let mut c = claims(&server.issuer);
    c["exp"] = json!(now() - 3600);
    let token = sign(&c, KEY_A_PEM, "kid-a");
    let err = verifier().verify(&token, &server.issuer).await.unwrap_err();
    assert!(matches!(err, AuthError::Expired), "{err:?}");
}

#[tokio::test]
async fn wrong_issuer_is_rejected() {
    let server = start().await;
    let mut c = claims(&server.issuer);
    c["iss"] = json!("http://evil.example/realms/other");
    let token = sign(&c, KEY_A_PEM, "kid-a");
    let err = verifier().verify(&token, &server.issuer).await.unwrap_err();
    assert!(matches!(err, AuthError::InvalidToken(_)), "{err:?}");
}

#[tokio::test]
async fn untrusted_client_is_rejected() {
    let server = start().await;
    let mut c = claims(&server.issuer);
    c["azp"] = json!("rogue-client");
    let token = sign(&c, KEY_A_PEM, "kid-a");
    let err = verifier().verify(&token, &server.issuer).await.unwrap_err();
    assert!(
        matches!(err, AuthError::UntrustedClient(ref id) if id == "rogue-client"),
        "{err:?}"
    );
}

#[tokio::test]
async fn tampered_signature_is_rejected() {
    let server = start().await;
    let token = sign(&claims(&server.issuer), KEY_A_PEM, "kid-a");
    let other = sign(&claims(&server.issuer), KEY_B_PEM, "kid-a");
    let signature = other.rsplit('.').next().unwrap();
    let body = token.rsplit_once('.').unwrap().0;
    let forged = format!("{body}.{signature}");
    let err = verifier()
        .verify(&forged, &server.issuer)
        .await
        .unwrap_err();
    assert!(matches!(err, AuthError::InvalidToken(_)), "{err:?}");
}

#[tokio::test]
async fn non_bearer_typ_is_rejected() {
    let server = start().await;
    let mut c = claims(&server.issuer);
    c["typ"] = json!("ID");
    let token = sign(&c, KEY_A_PEM, "kid-a");
    let err = verifier().verify(&token, &server.issuer).await.unwrap_err();
    assert!(matches!(err, AuthError::InvalidToken(_)), "{err:?}");
}

#[tokio::test]
async fn missing_kid_is_malformed() {
    let server = start().await;
    let key = EncodingKey::from_rsa_pem(KEY_A_PEM.as_bytes()).unwrap();
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::RS256),
        &claims(&server.issuer),
        &key,
    )
    .unwrap();
    let err = verifier().verify(&token, &server.issuer).await.unwrap_err();
    assert!(matches!(err, AuthError::Malformed(_)), "{err:?}");
}

#[tokio::test]
async fn rotated_key_triggers_refetch() {
    let server = start().await;
    let verifier = verifier();
    let first = sign(&claims(&server.issuer), KEY_A_PEM, "kid-a");
    verifier.verify(&first, &server.issuer).await.unwrap();

    server.served.lock().await.push(jwk(KEY_B_JWK, "kid-b"));
    let second = sign(&claims(&server.issuer), KEY_B_PEM, "kid-b");
    verifier.verify(&second, &server.issuer).await.unwrap();
    assert_eq!(server.jwks_hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn unknown_kid_is_not_found_and_refetch_is_throttled() {
    let server = start().await;
    let verifier = FerrisKeyVerifier::with_refresh_interval(
        AcceptedClients::new(["capitainerie-web"]),
        Duration::from_secs(3600),
    );
    let known = sign(&claims(&server.issuer), KEY_A_PEM, "kid-a");
    verifier.verify(&known, &server.issuer).await.unwrap();
    let unknown = sign(&claims(&server.issuer), KEY_B_PEM, "kid-b");
    for _ in 0..2 {
        let err = verifier.verify(&unknown, &server.issuer).await.unwrap_err();
        assert!(matches!(err, AuthError::KeyNotFound(_)), "{err:?}");
    }
    assert_eq!(server.jwks_hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn jwks_is_fetched_once_for_two_verifications() {
    let server = start().await;
    let verifier = FerrisKeyVerifier::new(AcceptedClients::new(["capitainerie-web"]));
    let token = sign(&claims(&server.issuer), KEY_A_PEM, "kid-a");
    verifier.verify(&token, &server.issuer).await.unwrap();
    verifier.verify(&token, &server.issuer).await.unwrap();
    assert_eq!(server.jwks_hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn delegated_token_with_accepted_actor_is_accepted() {
    let server = start().await;
    let mut c = claims(&server.issuer);
    c["azp"] = json!("some-other-client");
    c["act"] = json!({
        "sub": "agent-sub",
        "client_id": "capitainerie-agent",
        "act": {"sub": "inner", "client_id": "deep"},
    });
    let token = sign(&c, KEY_A_PEM, "kid-a");
    let identity = verifier().verify(&token, &server.issuer).await.unwrap();
    let actor = identity.actor.unwrap();
    assert_eq!(actor.sub, "agent-sub");
    assert_eq!(actor.client_id.as_deref(), Some("capitainerie-agent"));
    assert_eq!(actor.act.unwrap().sub, "inner");
}

#[tokio::test]
async fn delegated_token_with_untrusted_actor_and_azp_is_rejected() {
    let server = start().await;
    let mut c = claims(&server.issuer);
    c["azp"] = json!("some-other-client");
    c["act"] = json!({"sub": "agent-sub", "client_id": "rogue"});
    let token = sign(&c, KEY_A_PEM, "kid-a");
    let err = verifier().verify(&token, &server.issuer).await.unwrap_err();
    assert!(matches!(err, AuthError::UntrustedClient(_)), "{err:?}");
}

#[tokio::test]
async fn unreachable_issuer_is_discovery_error() {
    let token = sign(&claims("http://127.0.0.1:1/realms/x"), KEY_A_PEM, "kid-a");
    let err = verifier()
        .verify(&token, "http://127.0.0.1:1/realms/x")
        .await
        .unwrap_err();
    assert!(matches!(err, AuthError::Discovery(_)), "{err:?}");
}
