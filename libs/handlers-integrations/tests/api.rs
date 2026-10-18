#![allow(clippy::unwrap_used)]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Extension, Json, Router,
    extract::{Path, Request},
    http::StatusCode,
    routing::{get, post},
};
use capitainerie_auth::Identity;
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::agent::SecretToken,
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApps, GithubBases};
use capitainerie_handlers::Caller;
use capitainerie_handlers_integrations::{
    GithubIntegration,
    signer::{StatePurpose, StateSigner},
};
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

async fn fake_github(conversions: Arc<AtomicUsize>) -> String {
    let app = Router::new()
        .route(
            "/app-manifests/{code}/conversions",
            post(|Path(code): Path<String>| async move {
                conversions.fetch_add(1, Ordering::SeqCst);
                let Some(id) = code
                    .strip_prefix("code-")
                    .and_then(|id| id.parse::<i64>().ok())
                else {
                    return (StatusCode::NOT_FOUND, Json(json!({"message": "Not Found"})));
                };
                (
                    StatusCode::CREATED,
                    Json(json!({
                        "id": id,
                        "slug": "capitainerie-test",
                        "client_id": "Iv1.test",
                        "client_secret": "oauth-secret",
                        "pem": PEM,
                        "webhook_secret": "hook-secret",
                        "html_url": "https://github.com/apps/capitainerie-test",
                        "owner": {"login": "acme", "type": "Organization"},
                    })),
                )
            }),
        )
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

const CONSOLE: &str = "http://console.test";

struct Backend {
    base: String,
    conversions: Arc<AtomicUsize>,
}

async fn backend() -> Backend {
    let conversions = Arc::new(AtomicUsize::new(0));
    Backend {
        base: fake_github(Arc::clone(&conversions)).await,
        conversions,
    }
}

fn usecase_of(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[5u8; 32]).unwrap())
}

fn integration(
    usecase: &CapitainerieUseCase,
    backend: &Backend,
    signer: Option<StateSigner>,
    console_url: Option<&str>,
) -> GithubIntegration {
    let apps = GithubApps::new(
        usecase.clone(),
        GithubBases {
            api_base: backend.base.clone(),
            web_base: backend.base.clone(),
        },
    )
    .unwrap();
    GithubIntegration {
        apps: Arc::new(apps),
        signer,
        console_url: console_url.map(str::to_owned),
    }
}

async fn serve(pool: &PgPool, caller: Caller, backend: &Backend) -> String {
    serve_with(pool, caller, backend, Some(signer()), Some(CONSOLE)).await
}

async fn serve_with(
    pool: &PgPool,
    caller: Caller,
    backend: &Backend,
    signer: Option<StateSigner>,
    console_url: Option<&str>,
) -> String {
    let usecase = usecase_of(pool);
    let github = integration(&usecase, backend, signer, console_url);
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

async fn json_of(response: reqwest::Response) -> Value {
    response.json().await.unwrap()
}

async fn manifest_state(client: &reqwest::Client, admin: &str) -> String {
    let response = client
        .post(format!("{admin}/api/v1/integrations/github/app/manifest"))
        .json(&json!({"owner": {"type": "user"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    json_of(response).await["state"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn create_app_with(
    client: &reqwest::Client,
    admin: &str,
    state: &str,
    code: &str,
) -> reqwest::Response {
    client
        .post(format!("{admin}/api/v1/integrations/github/app"))
        .json(&json!({"code": code, "state": state}))
        .send()
        .await
        .unwrap()
}

async fn create_app(client: &reqwest::Client, admin: &str) -> reqwest::Response {
    let state = manifest_state(client, admin).await;
    create_app_with(
        client,
        admin,
        &state,
        &format!("code-{}", fresh_installation()),
    )
    .await
}

async fn install_state(client: &reqwest::Client, admin: &str, base: &str) -> String {
    let body: Value = client
        .get(format!("{admin}/api/v1/integrations/github/install"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let url = body["url"].as_str().unwrap();
    assert!(
        url.starts_with(&format!(
            "{base}/apps/capitainerie-test/installations/new?state="
        )),
        "{url}"
    );
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

fn install_signed(organization: &Organization, subject: &str, at: chrono::DateTime<Utc>) -> String {
    signer().sign(StatePurpose::Install, organization.id, subject, at)
}

async fn count(pool: &PgPool, table: &str, organization: &Organization) -> i64 {
    sqlx::query_scalar(&format!(
        "SELECT count(*) FROM {table} WHERE organization_id = $1"
    ))
    .bind(organization.id.0)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn admin_with_app(
    pool: &PgPool,
    organization: &Organization,
    backend: &Backend,
) -> (String, reqwest::Client) {
    let admin = serve(pool, caller(organization, "user-1", &[ADMIN]), backend).await;
    let client = reqwest::Client::new();
    assert_eq!(create_app(&client, &admin).await.status(), 201);
    (admin, client)
}

#[tokio::test]
async fn a_non_admin_is_refused_and_audited_before_anything_else_happens() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let member = serve_with(
        &pool,
        caller(&organization, "user-1", &[]),
        &backend,
        None,
        None,
    )
    .await;
    let client = reqwest::Client::new();
    let base = format!("{member}/api/v1/integrations/github");

    let refused = [
        client.get(format!("{base}/install")).send().await.unwrap(),
        client.delete(&base).send().await.unwrap(),
        client
            .post(format!("{base}/app/manifest"))
            .json(&json!({"owner": {"type": "organization", "login": "no/pe"}}))
            .send()
            .await
            .unwrap(),
        create_app_with(&client, &member, "garbage", "code-1").await,
        client.delete(format!("{base}/app")).send().await.unwrap(),
        connect(&client, &member, "garbage", 1).await,
    ];

    for response in &refused {
        assert_eq!(response.status(), 403);
    }
    assert_eq!(denied(&pool, &organization).await, 6);
    assert_eq!(backend.conversions.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn the_admin_creates_the_app_installs_it_and_a_member_reads_the_result() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let installation_id = fresh_installation();
    let admin = serve(&pool, caller(&organization, "user-1", &[ADMIN]), &backend).await;
    let member = serve(&pool, caller(&organization, "user-2", &[]), &backend).await;
    let client = reqwest::Client::new();
    let read = || async {
        json_of(
            client
                .get(format!("{member}/api/v1/integrations/github"))
                .send()
                .await
                .unwrap(),
        )
        .await
    };

    let before = read().await;
    let no_install = client
        .get(format!("{admin}/api/v1/integrations/github/install"))
        .send()
        .await
        .unwrap();
    let created = create_app(&client, &admin).await;
    let created_status = created.status();
    let created = json_of(created).await;
    let with_app = read().await;
    let state = install_state(&client, &admin, &backend.base).await;
    let connected = connect(&client, &admin, &state, installation_id).await;
    let connected_status = connected.status();
    let connected = json_of(connected).await;
    let installed = read().await;
    let repositories = json_of(
        client
            .get(format!("{member}/api/v1/integrations/github/repositories"))
            .send()
            .await
            .unwrap(),
    )
    .await;
    let disconnected = json_of(
        client
            .delete(format!("{admin}/api/v1/integrations/github"))
            .send()
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(before, json!({"app": null, "installation": null}));
    assert_eq!(no_install.status(), 409);
    assert_eq!(json_of(no_install).await["message"], "github_app_missing");
    assert_eq!(created_status, 201);
    assert_eq!(created["slug"], "capitainerie-test");
    assert_eq!(created["owner_login"], "acme");
    assert_eq!(created["owner_type"], "Organization");
    assert_eq!(created["created_by"], "user-1");
    assert_eq!(
        created["html_url"],
        "https://github.com/apps/capitainerie-test"
    );
    for secret in [
        "client_secret",
        "pem",
        "private_key",
        "webhook_secret",
        "client_id",
    ] {
        assert!(created.get(secret).is_none(), "{secret}");
    }
    assert_eq!(with_app["app"]["slug"], "capitainerie-test");
    assert!(with_app["installation"].is_null());
    assert_eq!(connected_status, 200);
    for body in [&connected, &installed] {
        assert_eq!(body["app"]["slug"], "capitainerie-test");
        assert_eq!(body["installation"]["account_login"], "acme");
        assert_eq!(body["installation"]["installation_id"], installation_id);
        assert_eq!(body["installation"]["connected_by"], "user-1");
        assert!(body["installation"]["connected_at"].is_string());
    }
    assert_eq!(repositories["repositories"][0]["full_name"], "acme/web");
    assert_eq!(repositories["repositories"][1]["name"], "api");
    assert_eq!(disconnected["app"]["slug"], "capitainerie-test");
    assert!(disconnected["installation"].is_null());
}

#[tokio::test]
async fn the_app_secrets_are_sealed_at_rest_and_open_for_the_system() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    admin_with_app(&pool, &organization, &backend).await;

    let (ciphertext, nonce): (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT private_key_ciphertext, private_key_nonce FROM github_apps WHERE organization_id = $1",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    let stored = usecase_of(&pool)
        .github_app_for(organization.id)
        .await
        .unwrap()
        .unwrap();

    assert!(!String::from_utf8_lossy(&ciphertext).contains("PRIVATE KEY"));
    assert_eq!(nonce.len(), 12);
    assert_eq!(stored.private_key_pem.expose(), PEM);
    assert_eq!(stored.client_secret.expose(), "oauth-secret");
    assert_eq!(stored.webhook_secret.expose(), "hook-secret");
    assert_eq!(stored.client_id, "Iv1.test");
    assert!(!format!("{stored:?}").contains("oauth-secret"));
}

#[tokio::test]
async fn an_organization_has_one_app_and_the_manifest_code_is_not_spent_on_a_second() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let (admin, client) = admin_with_app(&pool, &organization, &backend).await;
    assert_eq!(backend.conversions.load(Ordering::SeqCst), 1);
    let state = signer().sign(
        StatePurpose::Manifest,
        organization.id,
        "user-1",
        Utc::now(),
    );

    let again = create_app_with(&client, &admin, &state, "code-77").await;
    let manifest = client
        .post(format!("{admin}/api/v1/integrations/github/app/manifest"))
        .json(&json!({"owner": {"type": "user"}}))
        .send()
        .await
        .unwrap();

    assert_eq!(again.status(), 409);
    assert_eq!(json_of(again).await["message"], "github_app_exists");
    assert_eq!(manifest.status(), 409);
    assert_eq!(backend.conversions.load(Ordering::SeqCst), 1);
    assert_eq!(count(&pool, "github_apps", &organization).await, 1);
}

#[tokio::test]
async fn deleting_the_app_removes_the_installation_and_points_to_github() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let (admin, client) = admin_with_app(&pool, &organization, &backend).await;
    let state = install_state(&client, &admin, &backend.base).await;
    assert_eq!(
        connect(&client, &admin, &state, fresh_installation())
            .await
            .status(),
        200
    );
    assert_eq!(count(&pool, "github_installations", &organization).await, 1);

    let deleted = client
        .delete(format!("{admin}/api/v1/integrations/github/app"))
        .send()
        .await
        .unwrap();
    let status = deleted.status();
    let deleted = json_of(deleted).await;
    let after = json_of(
        client
            .get(format!("{admin}/api/v1/integrations/github"))
            .send()
            .await
            .unwrap(),
    )
    .await;
    let twice = client
        .delete(format!("{admin}/api/v1/integrations/github/app"))
        .send()
        .await
        .unwrap();

    assert_eq!(status, 200);
    assert_eq!(
        deleted["html_url"],
        "https://github.com/apps/capitainerie-test"
    );
    assert_eq!(after, json!({"app": null, "installation": null}));
    assert_eq!(count(&pool, "github_apps", &organization).await, 0);
    assert_eq!(count(&pool, "github_installations", &organization).await, 0);
    assert_eq!(twice.status(), 404);
    assert_eq!(
        create_app(&client, &admin).await.status(),
        201,
        "a new App can be created once the old one is deleted"
    );
}

#[tokio::test]
async fn a_state_cannot_cross_purposes() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let admin = serve(&pool, caller(&organization, "user-1", &[ADMIN]), &backend).await;
    let client = reqwest::Client::new();
    let install = install_signed(&organization, "user-1", Utc::now());

    let as_manifest = create_app_with(&client, &admin, &install, "code-1").await;
    assert_eq!(as_manifest.status(), 400);
    assert_eq!(json_of(as_manifest).await["code"], "invalid_state");
    assert_eq!(count(&pool, "github_apps", &organization).await, 0);
    assert_eq!(backend.conversions.load(Ordering::SeqCst), 0);

    let state = manifest_state(&client, &admin).await;
    assert_eq!(
        create_app_with(
            &client,
            &admin,
            &state,
            &format!("code-{}", fresh_installation())
        )
        .await
        .status(),
        201
    );
    let as_install = connect(&client, &admin, &state, fresh_installation()).await;
    assert_eq!(as_install.status(), 400);
    assert_eq!(json_of(as_install).await["code"], "invalid_state");
    assert_eq!(count(&pool, "github_installations", &organization).await, 0);
}

#[tokio::test]
async fn a_forged_expired_or_borrowed_manifest_state_is_refused_and_nothing_is_stored() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let admin = serve(&pool, caller(&organization, "user-1", &[ADMIN]), &backend).await;
    let client = reqwest::Client::new();
    let now = Utc::now();
    let sign = |signer: &StateSigner, org, subject, at| {
        signer.sign(StatePurpose::Manifest, org, subject, at)
    };
    let good = sign(&signer(), organization.id, "user-1", now);
    let (payload, tag) = good.split_once('.').unwrap();

    let refused = [
        sign(
            &signer(),
            organization.id,
            "user-1",
            now - Duration::minutes(11),
        ),
        sign(&signer(), OrganizationId::new(), "user-1", now),
        sign(&signer(), organization.id, "user-2", now),
        sign(
            &StateSigner::new(&[4u8; 32]).unwrap(),
            organization.id,
            "user-1",
            now,
        ),
        format!("{payload}x.{tag}"),
        format!("{payload}.{}", flip_first(tag)),
        "garbage".to_owned(),
    ];
    for state in &refused {
        let response = create_app_with(&client, &admin, state, "code-5").await;
        assert_eq!(response.status(), 400, "{state}");
        assert_eq!(json_of(response).await["code"], "invalid_state");
    }

    assert_eq!(count(&pool, "github_apps", &organization).await, 0);
    assert_eq!(backend.conversions.load(Ordering::SeqCst), 0);
    assert_eq!(
        create_app_with(
            &client,
            &admin,
            &good,
            &format!("code-{}", fresh_installation())
        )
        .await
        .status(),
        201
    );
}

#[tokio::test]
async fn a_code_github_refuses_stores_nothing() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let admin = serve(&pool, caller(&organization, "user-1", &[ADMIN]), &backend).await;
    let client = reqwest::Client::new();
    let state = manifest_state(&client, &admin).await;

    let expired = create_app_with(&client, &admin, &state, "expired").await;
    let malformed = create_app_with(&client, &admin, &state, "../x").await;

    assert_eq!(expired.status(), 422);
    assert_eq!(malformed.status(), 422);
    assert_eq!(count(&pool, "github_apps", &organization).await, 0);
}

#[tokio::test]
async fn the_manifest_targets_the_owner_and_the_console() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let admin = serve(&pool, caller(&organization, "user-1", &[ADMIN]), &backend).await;
    let client = reqwest::Client::new();
    let url = format!("{admin}/api/v1/integrations/github/app/manifest");

    let user = json_of(
        client
            .post(&url)
            .json(&json!({"owner": {"type": "user"}}))
            .send()
            .await
            .unwrap(),
    )
    .await;
    let org = json_of(
        client
            .post(&url)
            .json(&json!({"owner": {"type": "organization", "login": "acme-corp"}}))
            .send()
            .await
            .unwrap(),
    )
    .await;
    let bad = client
        .post(&url)
        .json(&json!({"owner": {"type": "organization", "login": "../evil"}}))
        .send()
        .await
        .unwrap();

    let state = user["state"].as_str().unwrap();
    assert_eq!(
        user["action_url"],
        format!("{}/settings/apps/new?state={state}", backend.base)
    );
    let state = org["state"].as_str().unwrap();
    assert_eq!(
        org["action_url"],
        format!(
            "{}/organizations/acme-corp/settings/apps/new?state={state}",
            backend.base
        )
    );
    let manifest: Value = serde_json::from_str(user["manifest"].as_str().unwrap()).unwrap();
    let name = manifest["name"].as_str().unwrap();
    assert!(name.starts_with("Capitainerie Test "), "{name}");
    assert!(name.chars().count() <= 34);
    assert_eq!(manifest["url"], CONSOLE);
    assert_eq!(
        manifest["redirect_url"],
        format!("{CONSOLE}/settings/integrations/github/app-callback")
    );
    assert_eq!(
        manifest["callback_urls"],
        json!([format!("{CONSOLE}/settings/integrations/github/callback")])
    );
    assert_eq!(manifest["public"], false);
    assert!(manifest.get("hook_attributes").is_none());
    assert_eq!(manifest["default_permissions"]["members"], "read");
    assert_eq!(bad.status(), 422);
}

#[tokio::test]
async fn without_a_console_url_or_a_secrets_key_the_app_cannot_be_started() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let client = reqwest::Client::new();
    let no_console = serve_with(
        &pool,
        caller(&organization, "user-1", &[ADMIN]),
        &backend,
        Some(signer()),
        None,
    )
    .await;
    let no_key = serve_with(
        &pool,
        caller(&organization, "user-1", &[ADMIN]),
        &backend,
        None,
        Some(CONSOLE),
    )
    .await;

    let console = client
        .post(format!(
            "{no_console}/api/v1/integrations/github/app/manifest"
        ))
        .json(&json!({"owner": {"type": "user"}}))
        .send()
        .await
        .unwrap();
    let key = client
        .post(format!("{no_key}/api/v1/integrations/github/app/manifest"))
        .json(&json!({"owner": {"type": "user"}}))
        .send()
        .await
        .unwrap();
    let status = json_of(
        client
            .get(format!("{no_key}/api/v1/integrations/github"))
            .send()
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(console.status(), 503);
    assert_eq!(json_of(console).await["code"], "console_url_not_configured");
    assert_eq!(key.status(), 503);
    assert_eq!(json_of(key).await["code"], "secrets_key_not_configured");
    assert_eq!(status, json!({"app": null, "installation": null}));
}

#[tokio::test]
async fn a_forged_expired_or_borrowed_install_state_is_refused_and_nothing_is_stored() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let (admin, client) = admin_with_app(&pool, &organization, &backend).await;
    let installation_id = fresh_installation();
    let now = Utc::now();
    let good = install_signed(&organization, "user-1", now);
    let (payload, tag) = good.split_once('.').unwrap();

    let refused = [
        install_signed(&organization, "user-1", now - Duration::minutes(11)),
        signer().sign(StatePurpose::Install, OrganizationId::new(), "user-1", now),
        install_signed(&organization, "user-2", now),
        StateSigner::new(&[4u8; 32]).unwrap().sign(
            StatePurpose::Install,
            organization.id,
            "user-1",
            now,
        ),
        format!("{payload}x.{tag}"),
        format!("{payload}.{}", flip_first(tag)),
        "garbage".to_owned(),
    ];
    for state in &refused {
        let response = connect(&client, &admin, state, installation_id).await;
        assert_eq!(response.status(), 400, "{state}");
        assert_eq!(json_of(response).await["code"], "invalid_state");
    }

    assert_eq!(count(&pool, "github_installations", &organization).await, 0);
    assert_eq!(
        connect(&client, &admin, &good, installation_id)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn connecting_without_an_app_is_a_conflict() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let admin = serve(&pool, caller(&organization, "user-1", &[ADMIN]), &backend).await;
    let client = reqwest::Client::new();

    let response = connect(
        &client,
        &admin,
        &install_signed(&organization, "user-1", Utc::now()),
        fresh_installation(),
    )
    .await;
    let repositories = client
        .get(format!("{admin}/api/v1/integrations/github/repositories"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 409);
    assert_eq!(json_of(response).await["message"], "github_app_missing");
    assert_eq!(repositories.status(), 409);
    assert_eq!(count(&pool, "github_installations", &organization).await, 0);
}

#[tokio::test]
async fn an_installation_the_admin_cannot_access_is_refused_and_nothing_is_stored() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let (admin, client) = admin_with_app(&pool, &organization, &backend).await;
    let installation_id = fresh_installation();
    let state = install_signed(&organization, "user-1", Utc::now());

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
            json_of(response).await["code"],
            "installation_not_accessible"
        );
    }
    assert_eq!(count(&pool, "github_installations", &organization).await, 0);
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
    assert_eq!(count(&pool, "github_installations", &organization).await, 1);
}

#[tokio::test]
async fn an_installation_bound_to_another_organization_is_a_conflict() {
    let Some(pool) = pool().await else { return };
    let first = organization(&pool).await;
    let second = organization(&pool).await;
    let backend = backend().await;
    let (first_admin, client) = admin_with_app(&pool, &first, &backend).await;
    let (second_admin, _) = admin_with_app(&pool, &second, &backend).await;
    let installation_id = fresh_installation();

    let bound = connect(
        &client,
        &first_admin,
        &install_signed(&first, "user-1", Utc::now()),
        installation_id,
    )
    .await;
    let refused = connect(
        &client,
        &second_admin,
        &install_signed(&second, "user-1", Utc::now()),
        installation_id,
    )
    .await;

    assert_eq!(bound.status(), 200);
    assert_eq!(refused.status(), 409);
    assert_eq!(
        json_of(refused).await["message"],
        "installation_already_connected"
    );
    assert_eq!(count(&pool, "github_installations", &second).await, 0);
    assert_eq!(count(&pool, "github_installations", &first).await, 1);
}

#[tokio::test]
async fn an_installation_of_an_organization_the_user_does_not_administer_is_refused() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let backend = backend().await;
    let (admin, client) = admin_with_app(&pool, &organization, &backend).await;
    let installation_id = fresh_installation();
    let state = install_signed(&organization, "user-1", Utc::now());

    let response = connect_with(
        &client,
        &admin,
        &state,
        installation_id,
        &format!("{installation_id},notadmin"),
    )
    .await;

    assert_eq!(response.status(), 403);
    assert_eq!(json_of(response).await["code"], "installation_not_owned");
    assert_eq!(count(&pool, "github_installations", &organization).await, 0);
}
