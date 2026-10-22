#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use axum::{Extension, Router};
use capitainerie_auth::Identity;
use capitainerie_common::{ClientId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        client::{Client, service::CreateClient},
        integration::{
            ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport,
            action::ActionRequest, ports::IntegrationConnector, service::CreateInstance,
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApps, GithubBases};
use capitainerie_handlers::Caller;
use capitainerie_handlers_integrations::GithubIntegration;
use capitainerie_integrations::{HttpConnector, NetworkPolicy};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const ADMIN: &str = "capitainerie-admin";
const KEY: &str = "sk-0123456789abcdef-supersecret-wxyz";

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
        created_at: Utc::now(),
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

fn subject(organization: &Organization, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id: organization.id,
        actor: None,
        is_admin,
    }
}

fn usecase_of(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[5u8; 32]).unwrap())
}

struct Healthy;

impl IntegrationConnector for Healthy {
    async fn probe(
        &self,
        _connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        Ok(ProbeReport {
            server_version: "19.0".into(),
            warning: None,
        })
    }
    async fn perform(
        &self,
        _connection: &InstanceConnection,
        _request: &ActionRequest,
    ) -> Result<serde_json::Value, ConnectorFailure> {
        Err(ConnectorFailure::NotSupported("not scripted".into()))
    }
}

async fn serve(pool: &PgPool, caller: Caller) -> String {
    let usecase = usecase_of(pool);
    let apps = GithubApps::new(
        usecase.clone(),
        GithubBases {
            api_base: "http://127.0.0.1:9".into(),
            web_base: "http://127.0.0.1:9".into(),
        },
    )
    .unwrap();
    let github = GithubIntegration::new(Arc::new(apps));
    let connector = HttpConnector::new(NetworkPolicy::default()).unwrap();
    let app: Router = capitainerie_handlers_integrations::router::<()>(usecase, github, connector)
        .layer(Extension(caller));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
}

async fn client(pool: &PgPool, organization: &Organization, name: &str) -> Client {
    usecase_of(pool)
        .create_client(
            subject(organization, false),
            CreateClient {
                name: name.into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap()
}

async fn instance(
    pool: &PgPool,
    organization: &Organization,
    client: &Client,
    name: &str,
) -> IntegrationInstance {
    usecase_of(pool)
        .create_integration_instance(
            subject(organization, true),
            CreateInstance {
                client_id: client.id,
                kind: "odoo_json2".into(),
                name: name.into(),
                environment: "production".into(),
                base_url: "https://odoo.acme.test".into(),
                database: None,
                login: None,
                secret: SecretToken::new(KEY),
            },
            &Healthy,
        )
        .await
        .unwrap()
}

fn mentions_a_secret_field(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key("secret") || map.values().any(mentions_a_secret_field)
        }
        Value::Array(items) => items.iter().any(mentions_a_secret_field),
        _ => false,
    }
}

fn integrations_of(base: &str, client_id: ClientId) -> String {
    format!("{base}/api/v1/clients/{client_id}/integrations")
}

fn creation(name: &str, base_url: &str) -> Value {
    json!({
        "kind": "odoo_json2",
        "name": name,
        "environment": "production",
        "base_url": base_url,
        "secret": KEY,
    })
}

#[tokio::test]
async fn a_member_lists_the_instances_of_a_client_without_any_secret() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let acme = client(&pool, &organization, "Acme").await;
    let globex = client(&pool, &organization, "Globex").await;
    instance(&pool, &organization, &acme, "Odoo prod").await;
    instance(&pool, &organization, &acme, "Odoo staging").await;
    instance(&pool, &organization, &globex, "Odoo prod").await;
    let member = serve(&pool, caller(&organization, &[])).await;

    let response = reqwest::get(integrations_of(&member, acme.id))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let text = response.text().await.unwrap();
    let body: Value = serde_json::from_str(&text).unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["name"], "Odoo prod");
    assert_eq!(items[1]["name"], "Odoo staging");
    assert_eq!(items[0]["kind"], "odoo_json2");
    assert_eq!(items[0]["environment"], "production");
    assert_eq!(items[0]["base_url"], "https://odoo.acme.test");
    assert_eq!(items[0]["secret_preview"], "…wxyz");
    assert_eq!(items[0]["server_version"], "19.0");
    assert_eq!(items[0]["client_id"], acme.id.to_string());
    assert!(!mentions_a_secret_field(&body));
    assert!(!text.contains("supersecret"));
}

#[tokio::test]
async fn listing_an_unknown_client_is_a_404() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let member = serve(&pool, caller(&organization, &[])).await;

    let response = reqwest::get(integrations_of(&member, ClientId::new()))
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn a_non_admin_cannot_add_change_check_or_archive_and_the_refusals_are_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let acme = client(&pool, &organization, "Acme").await;
    let existing = instance(&pool, &organization, &acme, "Odoo prod").await;
    let member = serve(&pool, caller(&organization, &[])).await;
    let http = reqwest::Client::new();
    let item = format!("{member}/api/v1/integration-instances/{}", existing.id);

    let responses = [
        http.post(integrations_of(&member, acme.id))
            .json(&creation("Odoo recette", "https://odoo2.acme.test"))
            .send()
            .await
            .unwrap(),
        http.patch(&item)
            .json(&json!({"name": "renamed"}))
            .send()
            .await
            .unwrap(),
        http.post(format!("{item}/check")).send().await.unwrap(),
        http.delete(&item).send().await.unwrap(),
    ];

    for response in responses {
        assert_eq!(response.status(), 403);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["code"], "forbidden");
    }
    let denials: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions
         WHERE organization_id = $1 AND action = 'integration.manage' AND NOT allowed",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denials, 4);
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM integration_instances WHERE organization_id = $1")
            .bind(organization.id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn an_admin_is_refused_invalid_fields_and_unknown_clients_before_any_connection() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let acme = client(&pool, &organization, "Acme").await;
    let admin = serve(&pool, caller(&organization, &[ADMIN])).await;
    let http = reqwest::Client::new();

    let bad_url = http
        .post(integrations_of(&admin, acme.id))
        .json(&creation("Odoo prod", "ftp://odoo.acme.test"))
        .send()
        .await
        .unwrap();
    let no_secret = http
        .post(integrations_of(&admin, acme.id))
        .json(&json!({
            "kind": "n8n",
            "name": "n8n prod",
            "environment": "production",
            "base_url": "https://n8n.acme.test",
            "secret": "",
        }))
        .send()
        .await
        .unwrap();
    let jsonrpc_without_login = http
        .post(integrations_of(&admin, acme.id))
        .json(&json!({
            "kind": "odoo_jsonrpc",
            "name": "Odoo 17",
            "environment": "staging",
            "base_url": "https://odoo17.acme.test",
            "database": "acme",
            "secret": KEY,
        }))
        .send()
        .await
        .unwrap();
    let unknown_client = http
        .post(integrations_of(&admin, ClientId::new()))
        .json(&creation("Odoo prod", "https://odoo.acme.test"))
        .send()
        .await
        .unwrap();

    for response in [bad_url, no_secret, jsonrpc_without_login] {
        assert_eq!(response.status(), 422);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["code"], "invalid");
    }
    assert_eq!(unknown_client.status(), 404);
}

#[tokio::test]
async fn an_address_the_network_policy_blocks_is_a_422_with_its_code() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let acme = client(&pool, &organization, "Acme").await;
    let admin = serve(&pool, caller(&organization, &[ADMIN])).await;

    let response = reqwest::Client::new()
        .post(integrations_of(&admin, acme.id))
        .json(&creation("Odoo local", "http://10.0.0.5"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], "network_blocked");
    assert_eq!(body["status"], 422);
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM integration_instances WHERE client_id = $1")
            .bind(acme.id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn an_admin_renames_an_instance_and_a_taken_name_is_a_409() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let acme = client(&pool, &organization, "Acme").await;
    let prod = instance(&pool, &organization, &acme, "Odoo prod").await;
    instance(&pool, &organization, &acme, "Odoo staging").await;
    let admin = serve(&pool, caller(&organization, &[ADMIN])).await;
    let http = reqwest::Client::new();
    let item = format!("{admin}/api/v1/integration-instances/{}", prod.id);

    let renamed = http
        .patch(&item)
        .json(&json!({"name": "Odoo production", "environment": "staging"}))
        .send()
        .await
        .unwrap();
    let taken = http
        .patch(&item)
        .json(&json!({"name": "Odoo staging"}))
        .send()
        .await
        .unwrap();
    let nothing = http.patch(&item).json(&json!({})).send().await.unwrap();
    let unknown = http
        .patch(format!(
            "{admin}/api/v1/integration-instances/{}",
            capitainerie_common::IntegrationInstanceId::new()
        ))
        .json(&json!({"name": "x"}))
        .send()
        .await
        .unwrap();

    assert_eq!(renamed.status(), 200);
    let body: Value = renamed.json().await.unwrap();
    assert_eq!(body["name"], "Odoo production");
    assert_eq!(body["environment"], "staging");
    assert_eq!(body["secret_preview"], "…wxyz");
    assert!(!mentions_a_secret_field(&body));
    assert_eq!(taken.status(), 409);
    let body: Value = taken.json().await.unwrap();
    assert_eq!(body["code"], "instance_name_taken");
    assert_eq!(nothing.status(), 422);
    assert_eq!(unknown.status(), 404);
}

#[tokio::test]
async fn deleting_archives_the_instance_and_the_list_hides_it_unless_asked() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let acme = client(&pool, &organization, "Acme").await;
    let prod = instance(&pool, &organization, &acme, "Odoo prod").await;
    let admin = serve(&pool, caller(&organization, &[ADMIN])).await;
    let http = reqwest::Client::new();
    let item = format!("{admin}/api/v1/integration-instances/{}", prod.id);

    let archived = http.delete(&item).send().await.unwrap();
    let again = http.delete(&item).send().await.unwrap();
    let active: Value = http
        .get(integrations_of(&admin, acme.id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let everything: Value = http
        .get(format!(
            "{}?archived=true",
            integrations_of(&admin, acme.id)
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let check = http.post(format!("{item}/check")).send().await.unwrap();

    assert_eq!(archived.status(), 200);
    let body: Value = archived.json().await.unwrap();
    assert!(body["archived_at"].is_string());
    assert_eq!(body["name"], "Odoo prod");
    assert_eq!(again.status(), 200);
    assert_eq!(active["items"].as_array().unwrap().len(), 0);
    assert_eq!(everything["items"].as_array().unwrap().len(), 1);
    assert_eq!(check.status(), 422);
}

#[tokio::test]
async fn checking_an_unknown_instance_is_a_404() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let admin = serve(&pool, caller(&organization, &[ADMIN])).await;

    let response = reqwest::Client::new()
        .post(format!(
            "{admin}/api/v1/integration-instances/{}/check",
            capitainerie_common::IntegrationInstanceId::new()
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}
