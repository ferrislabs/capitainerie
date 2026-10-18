#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::Identity;
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

struct FakeCatalog;

impl ModelCatalog for FakeCatalog {
    async fn list_models(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        match access.kind {
            ProviderKind::Anthropic => Ok(vec![CatalogModel {
                provider: ProviderKind::Anthropic,
                id: "claude-haiku-5-5".into(),
                display_name: "Claude Haiku 5.5".into(),
                chat: true,
                embeddings: false,
                context_window: Some(200_000),
                input_price_per_mtok_micros: Some(1_000_000),
                output_price_per_mtok_micros: Some(5_000_000),
            }]),
            _ => Err(CoreError::Internal("unreachable".into())),
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

fn caller(organization: &Organization, roles: &[&str]) -> Caller {
    Caller {
        identity: Identity {
            subject: "user-1".into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: None,
            roles: roles.iter().map(|role| (*role).to_owned()).collect(),
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

#[tokio::test]
async fn a_non_admin_cannot_put_a_provider_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let base = serve(&pool, caller(&organization, &[])).await;

    let response = reqwest::Client::new()
        .put(format!("{base}/api/v1/providers/anthropic"))
        .json(&json!({ "api_key": SECRET }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 403);
    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'provider.manage' AND NOT allowed",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denied, 1);
    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM provider_credentials WHERE organization_id = $1")
            .bind(organization.id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, 0);
}

#[tokio::test]
async fn an_admin_stores_a_key_and_no_response_ever_shows_it() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let admin = serve(&pool, caller(&organization, &["capitainerie-admin"])).await;
    let member = serve(&pool, caller(&organization, &[])).await;
    let client = reqwest::Client::new();

    let put = client
        .put(format!("{admin}/api/v1/providers/anthropic"))
        .json(&json!({ "api_key": SECRET }))
        .send()
        .await
        .unwrap();
    let put_status = put.status();
    let put_body = put.text().await.unwrap();
    let listed = client
        .get(format!("{member}/api/v1/providers"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(put_status, 200);
    for body in [&put_body, &listed] {
        assert!(!body.contains("topsecret"), "{body}");
        assert!(body.contains("…wxyz"), "{body}");
    }
    let listed: Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(listed["providers"][0]["kind"], "anthropic");
    assert_eq!(listed["providers"][0]["created_by"], "user-1");
}

#[tokio::test]
async fn models_assignments_and_test_follow_the_providers() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let admin = serve(&pool, caller(&organization, &["capitainerie-admin"])).await;
    let member = serve(&pool, caller(&organization, &[])).await;
    let client = reqwest::Client::new();
    client
        .put(format!("{admin}/api/v1/providers/anthropic"))
        .json(&json!({ "api_key": SECRET }))
        .send()
        .await
        .unwrap();
    client
        .put(format!("{admin}/api/v1/providers/openai_compatible"))
        .json(&json!({ "base_url": "http://localhost:11434/v1" }))
        .send()
        .await
        .unwrap();

    let models: Value = client
        .get(format!("{member}/api/v1/models"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tested: Value = client
        .post(format!("{admin}/api/v1/providers/anthropic/test"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tested_down: Value = client
        .post(format!("{admin}/api/v1/providers/openai_compatible/test"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let member_test = client
        .post(format!("{member}/api/v1/providers/anthropic/test"))
        .send()
        .await
        .unwrap();
    let before: Value = client
        .get(format!("{member}/api/v1/model-assignments"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let chat = json!({ "provider": "anthropic", "model": "claude-haiku-5-5" });
    let embeddings = json!({ "provider": "anthropic", "model": "claude-haiku-5-5" });
    let invalid = client
        .put(format!("{admin}/api/v1/model-assignments"))
        .json(
            &json!({ "extractor": chat, "judge": chat, "writer": chat, "embeddings": embeddings }),
        )
        .send()
        .await
        .unwrap();
    let member_put = client
        .put(format!("{member}/api/v1/model-assignments"))
        .json(
            &json!({ "extractor": chat, "judge": chat, "writer": chat, "embeddings": embeddings }),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(models["models"][0]["id"], "claude-haiku-5-5");
    assert_eq!(models["models"][0]["capabilities"]["chat"], true);
    assert_eq!(
        models["models"][0]["price"]["output_per_mtok_micros"],
        5_000_000
    );
    assert_eq!(models["errors"][0]["provider"], "openai_compatible");
    assert_eq!(tested["ok"], true);
    assert_eq!(tested_down["ok"], false);
    assert_eq!(member_test.status(), 403);
    assert_eq!(before["assignments"], Value::Null);
    assert_eq!(invalid.status(), 422);
    assert_eq!(member_put.status(), 403);
}
