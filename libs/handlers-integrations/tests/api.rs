#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::Request,
    routing::{get, post},
};
use capitainerie_auth::Identity;
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug, domain::agent::SecretToken,
    infrastructure::postgres,
};
use capitainerie_github::{GithubApp, GithubClient};
use capitainerie_handlers::Caller;
use capitainerie_handlers_integrations::{GithubIntegration, signer::StateSigner};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const PEM: &str = include_str!("../../github/tests/fixtures/test-only-app-key.pem");
const ADMIN: &str = "capitainerie-admin";

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

fn caller(organization: &Organization, subject: &str, roles: &[&str]) -> Caller {
    Caller {
        identity: Identity {
            subject: subject.into(),
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

async fn spawn(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
}

fn fresh_installation() -> i64 {
    (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64
}

fn code_for(installations: &[i64]) -> String {
    installations
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

async fn fake_github() -> String {
    let app = Router::new()
        .route(
            "/login/oauth/access_token",
            post(|Json(body): Json<Value>| async move {
                assert_eq!(body["client_id"], "Iv1.test");
                assert_eq!(body["client_secret"], "oauth-secret");
                Json(json!({"access_token": format!("ghu_{}", body["code"].as_str().unwrap())}))
            }),
        )
        .route(
            "/user/installations",
            get(|request: Request| async move {
                let token = request.headers()["authorization"].to_str().unwrap();
                let ids = token.trim_start_matches("Bearer ghu_");
                let installations: Vec<Value> = ids
                    .split(',')
                    .filter_map(|id| id.parse::<i64>().ok())
                    .map(|id| json!({"id": id}))
                    .collect();
                Json(json!({"total_count": installations.len(), "installations": installations}))
            }),
        )
        .route(
            "/app/installations/{id}",
            get(|| async { Json(json!({"account": {"login": "acme", "type": "Organization"}})) }),
        )
        .route(
            "/user/memberships/orgs/{org}",
            get(|request: Request| async move {
                let token = request.headers()["authorization"].to_str().unwrap();
                let role = if token.contains("notadmin") {
                    "member"
                } else {
                    "admin"
                };
                Json(json!({"role": role, "state": "active"}))
            }),
        )
        .route(
            "/app/installations/{id}/access_tokens",
            post(|| async {
                Json(json!({
                    "token": "ghs_fake",
                    "expires_at": (Utc::now() + Duration::hours(1)).to_rfc3339(),
                }))
            }),
        )
        .route(
            "/installation/repositories",
            get(|| async {
                Json(
                    json!({"repositories": [{"full_name": "acme/web"}, {"full_name": "acme/api"}]}),
                )
            }),
        );
    spawn(app).await
}

fn signer() -> StateSigner {
    StateSigner::new(&[3u8; 32]).unwrap()
}

async fn configured() -> GithubIntegration {
    let mut app = GithubApp::new(1, SecretToken::new(PEM));
    app.api_base = fake_github().await;
    app.oauth_base.clone_from(&app.api_base);
    app.client_id = Some("Iv1.test".into());
    app.client_secret = Some(SecretToken::new("oauth-secret"));
    GithubIntegration {
        client: Some(Arc::new(GithubClient::new(app).unwrap())),
        slug: Some("capitainerie-test".into()),
        signer: Some(signer()),
    }
}

async fn serve(pool: &PgPool, caller: Caller, github: GithubIntegration) -> String {
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone());
    let app: Router =
        capitainerie_handlers_integrations::router::<()>(usecase, github).layer(Extension(caller));
    spawn(app).await
}

async fn denied(pool: &PgPool, organization: &Organization) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'github.manage' AND NOT allowed",
    )
    .bind(organization.id.0)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn flip_first(tag: &str) -> String {
    let flipped = if tag.starts_with('A') { 'B' } else { 'A' };
    format!("{flipped}{}", &tag[1..])
}

async fn install_state(client: &reqwest::Client, admin: &str) -> String {
    let body: Value = client
        .get(format!("{admin}/api/v1/integrations/github/install"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let url = body["url"].as_str().unwrap();
    assert!(url.starts_with("https://github.com/apps/capitainerie-test/installations/new?state="));
    url.split("state=").nth(1).unwrap().to_owned()
}

async fn connect_with(
    client: &reqwest::Client,
    base: &str,
    state: &str,
    installation_id: i64,
    code: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/api/v1/integrations/github/callback"))
        .json(&json!({"installation_id": installation_id, "code": code, "state": state}))
        .send()
        .await
        .unwrap()
}

async fn connect(
    client: &reqwest::Client,
    base: &str,
    state: &str,
    installation_id: i64,
) -> reqwest::Response {
    connect_with(
        client,
        base,
        state,
        installation_id,
        &code_for(&[installation_id]),
    )
    .await
}

#[tokio::test]
async fn a_non_admin_cannot_start_the_installation_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let member = serve(
        &pool,
        caller(&organization, "user-1", &[]),
        configured().await,
    )
    .await;
    let client = reqwest::Client::new();

    let install = client
        .get(format!("{member}/api/v1/integrations/github/install"))
        .send()
        .await
        .unwrap();
    let disconnect = client
        .delete(format!("{member}/api/v1/integrations/github"))
        .send()
        .await
        .unwrap();

    assert_eq!(install.status(), 403);
    assert_eq!(disconnect.status(), 403);
    assert_eq!(denied(&pool, &organization).await, 2);
}

#[tokio::test]
async fn an_admin_connects_a_member_reads_and_lists_repositories() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let installation_id = fresh_installation();
    let github = configured().await;
    let admin = serve(
        &pool,
        caller(&organization, "user-1", &[ADMIN]),
        github.clone(),
    )
    .await;
    let member = serve(&pool, caller(&organization, "user-2", &[]), github).await;
    let client = reqwest::Client::new();

    let before: Value = client
        .get(format!("{member}/api/v1/integrations/github"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let no_repositories = client
        .get(format!("{member}/api/v1/integrations/github/repositories"))
        .send()
        .await
        .unwrap();
    let state = install_state(&client, &admin).await;
    let connected = connect(&client, &admin, &state, installation_id).await;
    let connected_status = connected.status();
    let connected: Value = connected.json().await.unwrap();
    let after: Value = client
        .get(format!("{member}/api/v1/integrations/github"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let repositories: Value = client
        .get(format!("{member}/api/v1/integrations/github/repositories"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let removed = client
        .delete(format!("{admin}/api/v1/integrations/github"))
        .send()
        .await
        .unwrap();
    let gone: Value = client
        .get(format!("{member}/api/v1/integrations/github"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(before["connected"], false);
    assert_eq!(before["app_configured"], true);
    assert_eq!(no_repositories.status(), 409);
    assert_eq!(
        no_repositories.json::<Value>().await.unwrap()["message"],
        "github_not_connected"
    );
    assert_eq!(connected_status, 200);
    assert_eq!(connected["connected"], true);
    for body in [&connected, &after] {
        assert_eq!(body["account_login"], "acme");
        assert_eq!(body["installation_id"], installation_id);
        assert_eq!(body["connected_by"], "user-1");
        assert!(body["connected_at"].is_string());
    }
    assert_eq!(repositories["repositories"][0]["full_name"], "acme/web");
    assert_eq!(repositories["repositories"][1]["owner"], "acme");
    assert_eq!(repositories["repositories"][1]["name"], "api");
    assert_eq!(removed.status(), 200);
    assert_eq!(gone["connected"], false);
}

#[tokio::test]
async fn a_forged_expired_or_borrowed_state_is_refused_and_nothing_is_stored() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let github = configured().await;
    let admin = serve(
        &pool,
        caller(&organization, "user-1", &[ADMIN]),
        github.clone(),
    )
    .await;
    let client = reqwest::Client::new();
    let installation_id = fresh_installation();
    let now = Utc::now();
    let good = signer().sign(organization.id, "user-1", now);
    let (payload, tag) = good.split_once('.').unwrap();

    let refused = [
        signer().sign(organization.id, "user-1", now - Duration::minutes(11)),
        signer().sign(OrganizationId::new(), "user-1", now),
        signer().sign(organization.id, "user-2", now),
        StateSigner::new(&[4u8; 32])
            .unwrap()
            .sign(organization.id, "user-1", now),
        format!("{payload}x.{tag}"),
        format!("{payload}.{}", flip_first(tag)),
        "garbage".to_owned(),
    ];
    for state in &refused {
        let response = connect(&client, &admin, state, installation_id).await;
        assert_eq!(response.status(), 400, "{state}");
        assert_eq!(
            response.json::<Value>().await.unwrap()["code"],
            "invalid_state"
        );
    }

    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM github_installations WHERE organization_id = $1")
            .bind(organization.id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, 0);
    assert_eq!(
        connect(&client, &admin, &good, installation_id)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn a_non_admin_with_a_valid_state_is_still_refused_by_the_policy() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let member = serve(
        &pool,
        caller(&organization, "user-2", &[]),
        configured().await,
    )
    .await;
    let state = signer().sign(organization.id, "user-2", Utc::now());

    let response = connect(
        &reqwest::Client::new(),
        &member,
        &state,
        fresh_installation(),
    )
    .await;

    assert_eq!(response.status(), 403);
    assert_eq!(denied(&pool, &organization).await, 1);
}

#[tokio::test]
async fn without_a_github_app_the_status_says_so_and_the_rest_answers_503() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let admin = serve(
        &pool,
        caller(&organization, "user-1", &[ADMIN]),
        GithubIntegration::default(),
    )
    .await;
    let client = reqwest::Client::new();

    let status: Value = client
        .get(format!("{admin}/api/v1/integrations/github"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let install = client
        .get(format!("{admin}/api/v1/integrations/github/install"))
        .send()
        .await
        .unwrap();
    let callback = connect(&client, &admin, "state", 1).await;
    let repositories = client
        .get(format!("{admin}/api/v1/integrations/github/repositories"))
        .send()
        .await
        .unwrap();

    assert_eq!(status["app_configured"], false);
    assert_eq!(status["connected"], false);
    for response in [install, callback, repositories] {
        assert_eq!(response.status(), 503);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["code"], "github_app_not_configured");
        assert!(body["message"].as_str().unwrap().contains("not configured"));
    }
}

async fn stored(pool: &PgPool, organization: &Organization) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM github_installations WHERE organization_id = $1")
        .bind(organization.id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn an_installation_the_admin_cannot_access_is_refused_and_nothing_is_stored() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let admin = serve(
        &pool,
        caller(&organization, "user-1", &[ADMIN]),
        configured().await,
    )
    .await;
    let client = reqwest::Client::new();
    let installation_id = fresh_installation();
    let state = signer().sign(organization.id, "user-1", Utc::now());

    let foreign = connect_with(
        &client,
        &admin,
        &state,
        installation_id,
        &code_for(&[installation_id + 1]),
    )
    .await;
    let nothing = connect_with(&client, &admin, &state, installation_id, "").await;

    for response in [foreign, nothing] {
        assert_eq!(response.status(), 403);
        assert_eq!(
            response.json::<Value>().await.unwrap()["code"],
            "installation_not_accessible"
        );
    }
    assert_eq!(stored(&pool, &organization).await, 0);
    assert_eq!(
        connect_with(
            &client,
            &admin,
            &state,
            installation_id,
            &code_for(&[1, installation_id]),
        )
        .await
        .status(),
        200
    );
    assert_eq!(stored(&pool, &organization).await, 1);
}

#[tokio::test]
async fn an_installation_bound_to_another_organization_is_a_conflict() {
    let Some(pool) = pool().await else { return };
    let first = organization(&pool).await;
    let second = organization(&pool).await;
    let github = configured().await;
    let first_admin = serve(&pool, caller(&first, "user-1", &[ADMIN]), github.clone()).await;
    let second_admin = serve(&pool, caller(&second, "user-1", &[ADMIN]), github).await;
    let client = reqwest::Client::new();
    let installation_id = fresh_installation();

    let bound = connect(
        &client,
        &first_admin,
        &signer().sign(first.id, "user-1", Utc::now()),
        installation_id,
    )
    .await;
    let refused = connect(
        &client,
        &second_admin,
        &signer().sign(second.id, "user-1", Utc::now()),
        installation_id,
    )
    .await;

    assert_eq!(bound.status(), 200);
    assert_eq!(refused.status(), 409);
    assert_eq!(
        refused.json::<Value>().await.unwrap()["message"],
        "installation_already_connected"
    );
    assert_eq!(stored(&pool, &second).await, 0);
    assert_eq!(stored(&pool, &first).await, 1);
}

#[tokio::test]
async fn a_non_admin_is_refused_by_the_policy_before_the_state_is_even_read() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let member = serve(
        &pool,
        caller(&organization, "user-2", &[]),
        configured().await,
    )
    .await;

    let response = connect(
        &reqwest::Client::new(),
        &member,
        "garbage",
        fresh_installation(),
    )
    .await;

    assert_eq!(response.status(), 403);
    assert_eq!(denied(&pool, &organization).await, 1);
}

#[tokio::test]
async fn an_installation_of_an_organization_the_user_does_not_administer_is_refused() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let admin = serve(
        &pool,
        caller(&organization, "user-1", &[ADMIN]),
        configured().await,
    )
    .await;
    let client = reqwest::Client::new();
    let installation_id = fresh_installation();
    let state = signer().sign(organization.id, "user-1", Utc::now());

    let response = connect_with(
        &client,
        &admin,
        &state,
        installation_id,
        &format!("{installation_id},notadmin"),
    )
    .await;

    assert_eq!(response.status(), 403);
    assert_eq!(
        response.json::<Value>().await.unwrap()["code"],
        "installation_not_owned"
    );
    assert_eq!(stored(&pool, &organization).await, 0);
}
