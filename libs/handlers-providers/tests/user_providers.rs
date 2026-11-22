#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{Actor, Identity};
use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        provider::{
            ProviderAccess, ProviderKind,
            ports::{CatalogModel, ModelCatalog},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::Caller;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const SECRET: &str = "sk-ant-api03-topsecretvalue-wxyz";
const GEMINI: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

struct FakeCatalog;

impl ModelCatalog for FakeCatalog {
    async fn list_models(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        match access.kind {
            ProviderKind::Anthropic => Ok(Vec::new()),
            ProviderKind::Mistral => Err(CoreError::Internal(format!(
                "401 unauthorized for key {}",
                access.secret.expose()
            ))),
            ProviderKind::OpenAi | ProviderKind::Google | ProviderKind::OpenAiCompatible => {
                Ok(Vec::new())
            }
        }
    }
}

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

async fn pool() -> Option<PgPool> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    Some(postgres::connect(&url, 2).await.unwrap())
}

async fn organization(pool: &PgPool) -> Organization {
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = format!("http://idp.test/realms/{id}");
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(&slug)
    .bind(&issuer_url)
    .execute(pool)
    .await
    .unwrap();
    Organization {
        id: OrganizationId(id),
        slug: OrganizationSlug::try_from(slug).unwrap(),
        name: "Test".into(),
        issuer_url,
        created_at: chrono::Utc::now(),
    }
}

fn caller(organization: &Organization, subject: &str, actor: Option<&str>) -> Caller {
    Caller {
        identity: Identity {
            subject: subject.into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: actor.map(|client_id| Actor {
                sub: "agent-sub".into(),
                client_id: Some(client_id.to_owned()),
                act: None,
            }),
            roles: Vec::new(),
        },
        organization: organization.clone(),
        token: SecretToken::new("token"),
    }
}

async fn serve(pool: &PgPool, caller: Caller) -> String {
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[9u8; 32]).unwrap());
    let app: Router = capitainerie_handlers_providers::router::<(), _>(usecase, FakeCatalog)
        .layer(Extension(caller));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
}

async fn put(base: &str, kind: &str, body: Value) -> (u16, Value) {
    let response = reqwest::Client::new()
        .put(format!("{base}/api/v1/me/providers/{kind}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    (
        response.status().as_u16(),
        response.json().await.unwrap_or(Value::Null),
    )
}

async fn list(base: &str) -> Value {
    reqwest::Client::new()
        .get(format!("{base}/api/v1/me/providers"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test]
async fn a_member_connects_a_provider_and_no_response_shows_the_key() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let base = serve(&pool, caller(&organization, "user-1", None)).await;

    let (status, stored) = put(
        &base,
        "anthropic",
        json!({"api_key": SECRET, "default_model": "claude-sonnet-5-5"}),
    )
    .await;
    let listed = list(&base).await;

    assert_eq!(status, 200);
    for body in [&stored, &listed[0]] {
        assert!(!body.to_string().contains("topsecret"), "{body}");
        assert_eq!(body["kind"], "anthropic");
        assert_eq!(body["preview"], "…wxyz");
        assert_eq!(body["default_model"], "claude-sonnet-5-5");
        assert_eq!(body["is_default"], true);
        assert_eq!(body["base_url"], Value::Null);
    }
    assert_eq!(listed.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn the_default_moves_on_request_and_follows_a_delete() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let base = serve(&pool, caller(&organization, "user-1", None)).await;
    put(
        &base,
        "anthropic",
        json!({"api_key": SECRET, "default_model": "claude-sonnet-5-5"}),
    )
    .await;
    let (_, second) = put(
        &base,
        "openai_compatible",
        json!({"base_url": GEMINI, "default_model": "gemini-3-flash", "is_default": true}),
    )
    .await;
    assert_eq!(second["is_default"], true);
    assert_eq!(second["base_url"], GEMINI);
    assert_eq!(second["preview"], "");

    let listed = list(&base).await;
    let defaults: Vec<_> = listed
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["is_default"] == true)
        .map(|entry| entry["kind"].clone())
        .collect();
    assert_eq!(defaults, [json!("openai_compatible")]);

    let deleted = reqwest::Client::new()
        .delete(format!("{base}/api/v1/me/providers/openai_compatible"))
        .send()
        .await
        .unwrap();
    assert_eq!(deleted.status(), 204);
    let listed = list(&base).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["kind"], "anthropic");
    assert_eq!(listed[0]["is_default"], true);

    let again = reqwest::Client::new()
        .delete(format!("{base}/api/v1/me/providers/openai_compatible"))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 404);
}

#[tokio::test]
async fn omitting_the_key_keeps_it_while_the_rest_changes() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let base = serve(&pool, caller(&organization, "user-1", None)).await;
    put(
        &base,
        "anthropic",
        json!({"api_key": SECRET, "default_model": "claude-sonnet-5-5"}),
    )
    .await;
    put(
        &base,
        "mistral",
        json!({"api_key": "sk-mistral-secret-9999", "default_model": "mistral-small"}),
    )
    .await;

    let (status, kept) = put(
        &base,
        "mistral",
        json!({"default_model": "mistral-large", "is_default": true}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(kept["preview"], "…9999");
    assert_eq!(kept["default_model"], "mistral-large");
    assert_eq!(kept["is_default"], true);
    let listed = list(&base).await;
    let defaults: Vec<_> = listed
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["is_default"] == true)
        .map(|entry| entry["kind"].clone())
        .collect();
    assert_eq!(defaults, [json!("mistral")]);

    let (status, again) = put(
        &base,
        "anthropic",
        json!({"default_model": "claude-haiku-5-5"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(again["preview"], "…wxyz");
    assert_eq!(again["is_default"], false);

    let client = reqwest::Client::new();
    let ok: Value = client
        .post(format!("{base}/api/v1/me/providers/anthropic/test"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ok["ok"], true);
    let failed: Value = client
        .post(format!("{base}/api/v1/me/providers/mistral/test"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(failed["error"].as_str().unwrap().contains("***"));
}

#[tokio::test]
async fn omitting_the_key_of_a_new_hosted_connection_is_a_422() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let base = serve(&pool, caller(&organization, "user-1", None)).await;
    for kind in ["anthropic", "mistral"] {
        let (status, _) = put(&base, kind, json!({"default_model": "some-model"})).await;
        assert_eq!(status, 422, "{kind}");
        let (status, _) = put(
            &base,
            kind,
            json!({"api_key": "", "default_model": "some-model"}),
        )
        .await;
        assert_eq!(status, 422, "{kind} empty");
    }
    assert_eq!(list(&base).await, json!([]));
}

#[tokio::test]
async fn invalid_input_is_a_422() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let base = serve(&pool, caller(&organization, "user-1", None)).await;
    for (kind, body) in [
        ("anthropic", json!({"default_model": "claude-sonnet-5-5"})),
        ("anthropic", json!({"api_key": SECRET, "default_model": ""})),
        (
            "anthropic",
            json!({"api_key": SECRET, "default_model": "Claude Sonnet"}),
        ),
        ("openai_compatible", json!({"default_model": "m"})),
        (
            "openai_compatible",
            json!({"base_url": "http://localhost:11434/v1", "default_model": "m"}),
        ),
        (
            "openai_compatible",
            json!({"base_url": "https://169.254.169.254/v1", "default_model": "m"}),
        ),
    ] {
        let (status, _) = put(&base, kind, body.clone()).await;
        assert_eq!(status, 422, "{kind} {body}");
    }
    assert_eq!(list(&base).await, json!([]));
}

#[tokio::test]
async fn users_and_organizations_never_see_each_others_connections() {
    let Some(pool) = pool().await else { return };
    let acme = organization(&pool).await;
    let other = organization(&pool).await;
    let alice = serve(&pool, caller(&acme, "alice", None)).await;
    let bob = serve(&pool, caller(&acme, "bob", None)).await;
    let outsider = serve(&pool, caller(&other, "alice", None)).await;
    put(
        &alice,
        "anthropic",
        json!({"api_key": SECRET, "default_model": "claude-sonnet-5-5"}),
    )
    .await;

    assert_eq!(list(&bob).await, json!([]));
    assert_eq!(list(&outsider).await, json!([]));
    let client = reqwest::Client::new();
    for base in [&bob, &outsider] {
        let tested = client
            .post(format!("{base}/api/v1/me/providers/anthropic/test"))
            .send()
            .await
            .unwrap();
        assert_eq!(tested.status(), 404);
        let deleted = client
            .delete(format!("{base}/api/v1/me/providers/anthropic"))
            .send()
            .await
            .unwrap();
        assert_eq!(deleted.status(), 404);
    }
    assert_eq!(list(&alice).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn an_agent_is_refused_on_every_route_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let agent = serve(
        &pool,
        caller(&organization, "user-1", Some("agent:darnold")),
    )
    .await;
    let client = reqwest::Client::new();

    let (put_status, _) = put(
        &agent,
        "anthropic",
        json!({"api_key": SECRET, "default_model": "claude-sonnet-5-5"}),
    )
    .await;
    let listed = client
        .get(format!("{agent}/api/v1/me/providers"))
        .send()
        .await
        .unwrap();
    let deleted = client
        .delete(format!("{agent}/api/v1/me/providers/anthropic"))
        .send()
        .await
        .unwrap();
    let tested = client
        .post(format!("{agent}/api/v1/me/providers/anthropic/test"))
        .send()
        .await
        .unwrap();

    assert_eq!(put_status, 403);
    assert_eq!(listed.status(), 403);
    assert_eq!(deleted.status(), 403);
    assert_eq!(tested.status(), 403);
    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'user_provider.manage' AND NOT allowed",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denied, 4);
    let stored: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM user_provider_credentials WHERE organization_id = $1",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, 0);
}

#[tokio::test]
async fn the_test_reports_ok_or_an_error_without_the_key() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let base = serve(&pool, caller(&organization, "user-1", None)).await;
    put(
        &base,
        "anthropic",
        json!({"api_key": SECRET, "default_model": "claude-sonnet-5-5"}),
    )
    .await;
    put(
        &base,
        "mistral",
        json!({"api_key": "sk-mistral-secret-9999", "default_model": "mistral-small"}),
    )
    .await;
    let client = reqwest::Client::new();

    let ok: Value = client
        .post(format!("{base}/api/v1/me/providers/anthropic/test"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let failed = client
        .post(format!("{base}/api/v1/me/providers/mistral/test"))
        .send()
        .await
        .unwrap();
    let failed_status = failed.status();
    let failed_text = failed.text().await.unwrap();
    let missing = client
        .post(format!("{base}/api/v1/me/providers/openai_compatible/test"))
        .send()
        .await
        .unwrap();

    assert_eq!(ok, json!({"ok": true, "error": null}));
    assert_eq!(failed_status, 200);
    assert!(
        !failed_text.contains("sk-mistral-secret-9999"),
        "{failed_text}"
    );
    let failed: Value = serde_json::from_str(&failed_text).unwrap();
    assert_eq!(failed["ok"], false);
    assert!(failed["error"].as_str().unwrap().contains("***"));
    assert_eq!(missing.status(), 404);
}
