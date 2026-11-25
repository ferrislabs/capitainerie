#![allow(clippy::unwrap_used)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch, post};
use axum::{Form, Json, Router};
use capitainerie_auth::{ClientCredentials, FerrisKeyProvisioner};
use capitainerie_common::{AgentId, CoreError};
use capitainerie_core::domain::agent::SecretToken;
use capitainerie_core::domain::agent_identity::AgentClientId;
use capitainerie_core::domain::agent_identity::ports::AgentIdentityProvisioner;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use uuid::Uuid;

const ADMIN_TOKEN: &str = "admin-token";

/// Client uuid, target audience, impersonation, delegation.
type Policy = (String, String, bool, bool);

#[derive(Clone, Default)]
struct Realm {
    clients: Arc<Mutex<HashMap<String, Value>>>,
    policies: Arc<Mutex<HashSet<Policy>>>,
    creations: Arc<Mutex<usize>>,
    forbid: bool,
}

impl Realm {
    async fn add(&self, client_id: &str, extra: Value) -> String {
        let id = Uuid::now_v7().to_string();
        let mut client = json!({
            "id": id,
            "client_id": client_id,
            "enabled": true,
            "public_client": false,
            "service_account_enabled": true,
            "token_exchange_enabled": true,
        });
        client
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        self.clients.lock().await.insert(id.clone(), client);
        id
    }

    async fn client(&self, client_id: &str) -> Option<Value> {
        self.clients
            .lock()
            .await
            .values()
            .find(|client| client["client_id"] == client_id)
            .cloned()
    }
}

#[derive(Clone)]
struct AppState {
    issuer: String,
    realm: Realm,
}

fn allowed(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    let bearer = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    if bearer != Some(&format!("Bearer {ADMIN_TOKEN}")) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if state.realm.forbid {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

async fn discovery(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "issuer": state.issuer,
        "token_endpoint": format!("{}/protocol/openid-connect/token", state.issuer),
    }))
}

async fn token(Form(form): Form<HashMap<String, String>>) -> (StatusCode, Json<Value>) {
    if form["grant_type"] == "client_credentials"
        && form["client_id"] == "capitainerie-provisioner"
        && form["client_secret"] == "provisioner-secret"
    {
        return (StatusCode::OK, Json(json!({"access_token": ADMIN_TOKEN})));
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "invalid_client"})),
    )
}

async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    allowed(&state, &headers)?;
    let found = state.realm.client(&query["client_id_exact"]).await;
    Ok(Json(json!({"data": found.into_iter().collect::<Vec<_>>()})))
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    allowed(&state, &headers)?;
    *state.realm.creations.lock().await += 1;
    let client_id = body["client_id"].as_str().unwrap().to_owned();
    let mut fields = body.clone();
    fields.as_object_mut().unwrap().remove("client_id");
    let id = state.realm.add(&client_id, fields).await;
    let client = state.realm.clients.lock().await[&id].clone();
    Ok((StatusCode::CREATED, Json(client)))
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    allowed(&state, &headers)?;
    let mut clients = state.realm.clients.lock().await;
    let client = clients.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    client
        .as_object_mut()
        .unwrap()
        .extend(body.as_object().unwrap().clone());
    Ok(Json(json!({"data": client})))
}

async fn policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<StatusCode, StatusCode> {
    allowed(&state, &headers)?;
    let target = body["target_audience"].as_str().unwrap().to_owned();
    let mut policies = state.realm.policies.lock().await;
    if policies
        .iter()
        .any(|(client, audience, _, _)| *client == id && *audience == target)
    {
        return Ok(StatusCode::CONFLICT);
    }
    policies.insert((
        id,
        target,
        body["allow_impersonation"].as_bool().unwrap(),
        body["allow_delegation"].as_bool().unwrap(),
    ));
    Ok(StatusCode::CREATED)
}

async fn secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    allowed(&state, &headers)?;
    Ok(Json(json!({"client_secret": format!("secret-of-{id}")})))
}

async fn start(realm: Realm) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}/realms/acme", listener.local_addr().unwrap());
    let state = AppState {
        issuer: issuer.clone(),
        realm,
    };
    let app = Router::new()
        .route(
            "/realms/acme/.well-known/openid-configuration",
            get(discovery),
        )
        .route("/realms/acme/protocol/openid-connect/token", post(token))
        .route("/realms/acme/clients", get(list).post(create))
        .route("/realms/acme/clients/{id}", patch(update))
        .route(
            "/realms/acme/clients/{id}/token-exchange-policies",
            post(policy),
        )
        .route("/realms/acme/clients/{id}/client-secret", get(secret))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    issuer
}

fn provisioner(secret: &str) -> FerrisKeyProvisioner {
    FerrisKeyProvisioner::new(
        ClientCredentials {
            client_id: "capitainerie-provisioner".into(),
            client_secret: SecretToken::new(secret),
        },
        "capitainerie-api",
        "capitainerie-mcp",
    )
}

async fn realm_with_api() -> (Realm, String) {
    let realm = Realm::default();
    let api = realm
        .add(
            "capitainerie-api",
            json!({"service_account_enabled": false}),
        )
        .await;
    (realm, api)
}

fn agent_client() -> AgentClientId {
    AgentClientId::of(AgentId(Uuid::from_u128(42)))
}

#[tokio::test]
async fn a_new_agent_gets_a_confidential_client_and_both_exchange_policies() {
    let (realm, api) = realm_with_api().await;
    let issuer = start(realm.clone()).await;
    let client_id = agent_client();

    let provisioned = provisioner("provisioner-secret")
        .provision(&issuer, &client_id, "Capitainerie agent Bernard")
        .await
        .unwrap();

    let client = realm.client(client_id.as_str()).await.unwrap();
    assert_eq!(client["id"], provisioned.client_uuid);
    assert_eq!(client["client_type"], "confidential");
    assert_eq!(client["service_account_enabled"], true);
    assert_eq!(client["token_exchange_enabled"], true);
    assert_eq!(client["direct_access_grants_enabled"], false);
    assert_eq!(client["name"], "Capitainerie agent Bernard");
    assert_eq!(
        provisioned.secret.expose(),
        format!("secret-of-{}", provisioned.client_uuid)
    );
    let policies = realm.policies.lock().await;
    assert!(policies.contains(&(
        provisioned.client_uuid.clone(),
        "capitainerie-mcp".into(),
        false,
        true
    )));
    assert!(policies.contains(&(api, client_id.as_str().into(), true, false)));
    assert_eq!(policies.len(), 2);
}

#[tokio::test]
async fn provisioning_twice_converges() {
    let (realm, _) = realm_with_api().await;
    let issuer = start(realm.clone()).await;
    let provisioner = provisioner("provisioner-secret");

    let first = provisioner
        .provision(&issuer, &agent_client(), "Bernard")
        .await
        .unwrap();
    let second = provisioner
        .provision(&issuer, &agent_client(), "Bernard")
        .await
        .unwrap();

    assert_eq!(first, second);
    assert_eq!(*realm.creations.lock().await, 1);
    assert_eq!(realm.policies.lock().await.len(), 2);
}

#[tokio::test]
async fn a_disabled_client_left_behind_is_turned_back_on() {
    let (realm, _) = realm_with_api().await;
    let client_id = agent_client();
    let existing = realm
        .add(
            client_id.as_str(),
            json!({"enabled": false, "token_exchange_enabled": false}),
        )
        .await;
    let issuer = start(realm.clone()).await;

    let provisioned = provisioner("provisioner-secret")
        .provision(&issuer, &client_id, "Bernard")
        .await
        .unwrap();

    assert_eq!(provisioned.client_uuid, existing);
    let client = realm.client(client_id.as_str()).await.unwrap();
    assert_eq!(client["enabled"], true);
    assert_eq!(client["token_exchange_enabled"], true);
    assert_eq!(*realm.creations.lock().await, 0);
}

#[tokio::test]
async fn a_public_client_with_the_same_id_is_not_taken_over() {
    let (realm, _) = realm_with_api().await;
    let client_id = agent_client();
    realm
        .add(client_id.as_str(), json!({"public_client": true}))
        .await;
    let issuer = start(realm.clone()).await;

    let refused = provisioner("provisioner-secret")
        .provision(&issuer, &client_id, "Bernard")
        .await;

    assert!(matches!(refused, Err(CoreError::Conflict(_))));
    assert!(realm.policies.lock().await.is_empty());
}

#[tokio::test]
async fn a_provisioner_without_the_permission_is_refused() {
    let (realm, _) = realm_with_api().await;
    let realm = Realm {
        forbid: true,
        ..realm
    };
    let issuer = start(realm).await;

    let refused = provisioner("provisioner-secret")
        .provision(&issuer, &agent_client(), "Bernard")
        .await;

    assert!(matches!(refused, Err(CoreError::Forbidden { .. })));
}

#[tokio::test]
async fn a_wrong_provisioner_secret_is_refused_without_leaking_it() {
    let (realm, _) = realm_with_api().await;
    let issuer = start(realm).await;

    let refused = provisioner("wrong-secret")
        .provision(&issuer, &agent_client(), "Bernard")
        .await
        .unwrap_err();

    assert!(matches!(refused, CoreError::Forbidden { .. }));
    assert!(!format!("{refused:?}").contains("wrong-secret"));
}

#[tokio::test]
async fn set_enabled_patches_the_client() {
    let (realm, _) = realm_with_api().await;
    let client = realm.add("capitainerie-agent-x", json!({})).await;
    let issuer = start(realm.clone()).await;

    provisioner("provisioner-secret")
        .set_enabled(&issuer, &client, false)
        .await
        .unwrap();

    assert_eq!(realm.clients.lock().await[&client]["enabled"], false);
}

async fn client_credentials(issuer: &str, client_id: &str, secret: &str) -> StatusCode {
    let status = reqwest::Client::new()
        .post(format!("{issuer}/protocol/openid-connect/token"))
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", secret),
        ])
        .send()
        .await
        .unwrap()
        .status();
    StatusCode::from_u16(status.as_u16()).unwrap()
}

/// Against a real FerrisKey bootstrapped by `scripts/bootstrap-ferriskey.sh`:
/// `FERRISKEY_ISSUER=http://localhost:3333/realms/capitainerie
/// PROVISIONER_CLIENT_SECRET=... cargo test -p capitainerie-auth --test
/// provisioning -- --ignored`. It leaves one agent client in the realm.
#[tokio::test]
#[ignore = "needs a running FerrisKey"]
async fn a_real_ferriskey_provisions_an_agent_that_can_sign_in() {
    let issuer = std::env::var("FERRISKEY_ISSUER").unwrap();
    let secret = std::env::var("PROVISIONER_CLIENT_SECRET").unwrap();
    let provisioner = provisioner(&secret);
    let client_id = AgentClientId::of(AgentId(Uuid::now_v7()));

    let first = provisioner
        .provision(&issuer, &client_id, "Capitainerie agent live test")
        .await
        .unwrap();
    let second = provisioner
        .provision(&issuer, &client_id, "Capitainerie agent live test")
        .await
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        client_credentials(&issuer, client_id.as_str(), first.secret.expose()).await,
        StatusCode::OK
    );

    provisioner
        .set_enabled(&issuer, &first.client_uuid, false)
        .await
        .unwrap();
    assert_ne!(
        client_credentials(&issuer, client_id.as_str(), first.secret.expose()).await,
        StatusCode::OK
    );
}
