#![allow(clippy::unwrap_used)]

#[path = "../../integrations/tests/mcp/fake.rs"]
mod fake;

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, Actor, FerrisKeyVerifier, Identity};
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::agent::SecretToken,
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::{AppState, Caller};
use capitainerie_integrations::{McpHttp, NetworkPolicy};
use chrono::Utc;
use fake::{Fake, Options};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const CONSOLE: &str = "http://console.test";

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

async fn organization(pool: &PgPool) -> Organization {
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = format!("http://idp.test/realms/{slug}");
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

#[derive(Clone, Copy)]
struct Setup<'a> {
    subject: &'a str,
    actor: Option<&'a str>,
    keyed: bool,
    console: bool,
}

const USER: Setup<'static> = Setup {
    subject: "user-1",
    actor: None,
    keyed: true,
    console: true,
};

struct Client {
    base: String,
}

async fn serve(pool: &PgPool, organization: &Organization, setup: Setup<'_>) -> Client {
    let mut usecase = CapitainerieUseCase::with_local_policies(pool.clone());
    if setup.keyed {
        usecase = usecase.with_secrets_cipher(SecretCipher::new(&[7u8; 32]).unwrap());
    }
    let state = AppState {
        usecase: usecase.clone(),
        verifier: FerrisKeyVerifier::new(AcceptedClients::new(["capitainerie-console"])),
        delegation: None,
    };
    let caller = Caller {
        identity: Identity {
            subject: setup.subject.into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: setup.actor.map(|client_id| Actor {
                sub: "service-account".into(),
                client_id: Some(client_id.to_owned()),
                act: None,
            }),
            roles: vec![],
        },
        organization: organization.clone(),
        token: SecretToken::new("user-token"),
    };
    let adapter = McpHttp::new(NetworkPolicy {
        allow_insecure_http: true,
        allow_private_networks: true,
    })
    .unwrap();
    let app: Router = capitainerie_handlers_mcp::router(
        usecase,
        adapter,
        setup.console.then(|| CONSOLE.to_owned()),
    )
    .layer(Extension(caller))
    .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Client {
        base: format!("http://{address}"),
    }
}

impl Client {
    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut request = reqwest::Client::new().request(method, format!("{}{path}", self.base));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        let body = response.json().await.unwrap_or(Value::Null);
        (status, body)
    }

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::GET, path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, path, Some(body)).await
    }

    async fn patch(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::PATCH, path, Some(body)).await
    }

    async fn delete(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::DELETE, path, None).await
    }
}

struct Fixture {
    pool: PgPool,
    organization: Organization,
    fake: Fake,
    client: Client,
}

async fn fixture() -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 4).await.unwrap();
    let organization = organization(&pool).await;
    let fake = fake::start(Options::default()).await;
    let client = serve(&pool, &organization, USER).await;
    Some(Fixture {
        pool,
        organization,
        fake,
        client,
    })
}

impl Fixture {
    async fn create(&self, name: &str) -> Value {
        let (status, created) = self
            .client
            .post(
                "/api/v1/mcp-connectors",
                json!({"url": format!("{}/mcp", self.fake.base), "name": name}),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        created
    }

    async fn browse(&self, authorization_url: &str) -> (String, String) {
        let response = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap()
            .get(authorization_url)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        let location = response.headers()["location"].to_str().unwrap().to_owned();
        let location = url::Url::parse(&location).unwrap();
        assert_eq!(
            format!("{}://{}", location.scheme(), location.host_str().unwrap()),
            CONSOLE
        );
        assert_eq!(location.path(), "/connectors/oauth/callback");
        let pair = |name: &str| {
            location
                .query_pairs()
                .find(|(key, _)| key == name)
                .unwrap()
                .1
                .into_owned()
        };
        (pair("state"), pair("code"))
    }

    async fn authorize(&self, id: &str) -> (String, String) {
        let (status, body) = self
            .client
            .post(&format!("/api/v1/mcp-connectors/{id}/authorize"), json!({}))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        self.browse(body["authorization_url"].as_str().unwrap())
            .await
    }

    async fn connect(&self, id: &str) {
        let (state, code) = self.authorize(id).await;
        let (status, body) = self
            .client
            .post(
                "/api/v1/mcp-connectors/oauth/callback",
                json!({"state": state, "code": code}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["connector_id"], id);
    }
}

#[tokio::test]
async fn a_connector_is_created_authorized_tested_refreshed_and_disconnected() {
    let Some(fixture) = fixture().await else {
        return;
    };

    let created = fixture.create("Mestier").await;
    assert_eq!(created["slug"], "mestier");
    assert_eq!(created["connected"], false);
    assert_eq!(created["expires_at"], Value::Null);
    assert_eq!(created["resource"], format!("{}/mcp", fixture.fake.base));
    assert_eq!(
        created["scopes"],
        json!(["mestier.read", "mestier.doe.write"])
    );
    assert!(created.get("client_id").is_none());
    let id = created["id"].as_str().unwrap().to_owned();
    let registration = fixture
        .fake
        .requests("POST", "/realms/mestier/clients/register")[0]
        .json();
    assert_eq!(
        registration["redirect_uris"],
        json!([format!("{CONSOLE}/connectors/oauth/callback")])
    );

    let (_, listed) = fixture.client.get("/api/v1/mcp-connectors").await;
    assert_eq!(listed["items"][0]["connected"], false);
    let (status, untested) = fixture
        .client
        .post(&format!("/api/v1/mcp-connectors/{id}/test"), json!({}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{untested}");

    fixture.connect(&id).await;
    let (_, listed) = fixture.client.get("/api/v1/mcp-connectors").await;
    assert_eq!(listed["items"][0]["connected"], true);
    assert!(listed["items"][0]["expires_at"].is_string());
    let (_, detail) = fixture
        .client
        .get(&format!("/api/v1/mcp-connectors/{id}"))
        .await;
    assert_eq!(detail["connected"], true);

    let (status, tested) = fixture
        .client
        .post(&format!("/api/v1/mcp-connectors/{id}/test"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK, "{tested}");
    assert_eq!(
        tested["tools"],
        json!([
            {"name": "list_doe", "read_only": true},
            {"name": "write_doe", "read_only": false},
            {"name": "bare", "read_only": false},
        ])
    );
    assert_eq!(fixture.fake.issued_tokens(), 1);

    sqlx::query("UPDATE mcp_connections SET expires_at = now() + interval '20 seconds' WHERE connector_id = $1::uuid")
        .bind(&id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let (status, refreshed) = fixture
        .client
        .post(&format!("/api/v1/mcp-connectors/{id}/test"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK, "{refreshed}");
    assert_eq!(
        fixture.fake.issued_tokens(),
        2,
        "the near-expiry token was refreshed"
    );
    let authorized = fixture.fake.requests("POST", "/mcp");
    assert_eq!(
        authorized.last().unwrap().authorization.as_deref(),
        Some("Bearer at-2")
    );

    let (status, _) = fixture
        .client
        .delete(&format!("/api/v1/mcp-connectors/{id}/connection"))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = fixture
        .client
        .delete(&format!("/api/v1/mcp-connectors/{id}/connection"))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, listed) = fixture.client.get("/api/v1/mcp-connectors").await;
    assert_eq!(listed["items"][0]["connected"], false);
    let (status, _) = fixture
        .client
        .post(&format!("/api/v1/mcp-connectors/{id}/test"), json!({}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_revoked_session_asks_to_reconnect_and_a_state_works_once() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let id = fixture.create("Mestier").await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (state, code) = fixture.authorize(&id).await;
    let body = json!({"state": state, "code": code});

    let (status, first) = fixture
        .client
        .post("/api/v1/mcp-connectors/oauth/callback", body.clone())
        .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (status, replay) = fixture
        .client
        .post("/api/v1/mcp-connectors/oauth/callback", body)
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{replay}");

    fixture.fake.expire_access_token();
    let (status, rejected) = fixture
        .client
        .post(&format!("/api/v1/mcp-connectors/{id}/test"), json!({}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{rejected}");
    assert!(!rejected.to_string().contains("at-1"));
}

#[tokio::test]
async fn another_user_cannot_complete_a_state_that_is_not_theirs() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let id = fixture.create("Mestier").await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (state, code) = fixture.authorize(&id).await;
    let intruder = serve(
        &fixture.pool,
        &fixture.organization,
        Setup {
            subject: "user-2",
            ..USER
        },
    )
    .await;

    let (status, body) = intruder
        .post(
            "/api/v1/mcp-connectors/oauth/callback",
            json!({"state": state, "code": code}),
        )
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (_, theirs) = intruder.get("/api/v1/mcp-connectors").await;
    assert_eq!(theirs["items"][0]["connected"], false);
    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mcp_connections WHERE connector_id = $1::uuid")
            .bind(&id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(stored, 0);
}

#[tokio::test]
async fn a_connector_of_another_organization_is_a_404_not_a_403() {
    let Some(mine) = fixture().await else { return };
    let Some(theirs) = fixture().await else {
        return;
    };
    let foreign = theirs.create("Mestier").await;
    let id = foreign["id"].as_str().unwrap();
    let (state, code) = theirs.authorize(id).await;
    let base = format!("/api/v1/mcp-connectors/{id}");

    assert_eq!(mine.client.get(&base).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        mine.client.patch(&base, json!({"name": "Pris"})).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        mine.client
            .post(&format!("{base}/authorize"), json!({}))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        mine.client.post(&format!("{base}/test"), json!({})).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        mine.client.delete(&format!("{base}/connection")).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, _) = mine
        .client
        .post(
            "/api/v1/mcp-connectors/oauth/callback",
            json!({"state": state, "code": code}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, list) = mine.client.get("/api/v1/mcp-connectors").await;
    assert!(list["items"].as_array().unwrap().is_empty());
    let (_, untouched) = theirs.client.get(&base).await;
    assert_eq!(untouched["name"], "Mestier");
}

#[tokio::test]
async fn an_agent_caller_is_refused_everywhere() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let id = fixture.create("Mestier").await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let agent = serve(
        &fixture.pool,
        &fixture.organization,
        Setup {
            actor: Some("capitainerie-agent-writer"),
            ..USER
        },
    )
    .await;
    let base = format!("/api/v1/mcp-connectors/{id}");
    let probes_before = fixture.fake.total_requests();

    for (method, path, body) in [
        (Method::GET, "/api/v1/mcp-connectors".to_owned(), None),
        (
            Method::POST,
            "/api/v1/mcp-connectors".to_owned(),
            Some(json!({"url": format!("{}/mcp", fixture.fake.base), "slug": "other"})),
        ),
        (Method::GET, base.clone(), None),
        (Method::PATCH, base.clone(), Some(json!({"archived": true}))),
        (Method::POST, format!("{base}/authorize"), Some(json!({}))),
        (
            Method::POST,
            "/api/v1/mcp-connectors/oauth/callback".to_owned(),
            Some(json!({"state": "x", "code": "y"})),
        ),
        (Method::DELETE, format!("{base}/connection"), None),
        (Method::POST, format!("{base}/test"), Some(json!({}))),
    ] {
        let (status, response) = agent.call(method.clone(), &path, body).await;
        assert!(
            status == StatusCode::FORBIDDEN || status == StatusCode::UNPROCESSABLE_ENTITY,
            "{method} {path}: {status} {response}"
        );
        if path != "/api/v1/mcp-connectors/oauth/callback" {
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
        }
    }
    assert_eq!(
        fixture.fake.total_requests(),
        probes_before,
        "a denied agent triggered no outbound call"
    );
    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mcp_connectors WHERE organization_id = $1")
            .bind(fixture.organization.id.0)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(stored, 1);
}

#[tokio::test]
async fn invalid_input_duplicates_and_empty_patches_are_refused_before_any_request() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let created = fixture.create("Mestier").await;
    let path = format!("/api/v1/mcp-connectors/{}", created["id"].as_str().unwrap());
    let before = fixture.fake.total_requests();

    for body in [
        json!({"url": "ftp://x"}),
        json!({"url": "https://u:p@x.example/mcp"}),
        json!({"url": format!("{}/mcp", fixture.fake.base), "slug": "Not Kebab"}),
        json!({"url": format!("{}/mcp", fixture.fake.base), "name": " "}),
    ] {
        let (status, _) = fixture
            .client
            .post("/api/v1/mcp-connectors", body.clone())
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
    let (status, _) = fixture
        .client
        .post(
            "/api/v1/mcp-connectors",
            json!({"url": format!("{}/mcp", fixture.fake.base), "name": "Mestier"}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(fixture.fake.total_requests(), before);

    assert_eq!(
        fixture.client.patch(&path, json!({})).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, renamed) = fixture
        .client
        .patch(&path, json!({"name": "ERP Mestier"}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["name"], "ERP Mestier");
    assert_eq!(renamed["slug"], "mestier");
    let (_, archived) = fixture.client.patch(&path, json!({"archived": true})).await;
    assert_eq!(archived["archived"], true);
    let (status, _) = fixture
        .client
        .post(&format!("{path}/authorize"), json!({}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, active) = fixture.client.get("/api/v1/mcp-connectors").await;
    assert!(active["items"].as_array().unwrap().is_empty());
    let (_, all) = fixture
        .client
        .get("/api/v1/mcp-connectors?archived=true")
        .await;
    assert_eq!(all["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_server_that_cannot_be_discovered_is_a_422_and_stores_nothing() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let bare = fake::start(Options {
        no_resource_metadata: true,
        ..Options::default()
    })
    .await;

    let (status, body) = fixture
        .client
        .post(
            "/api/v1/mcp-connectors",
            json!({"url": format!("{}/mcp", bare.base)}),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mcp_connectors WHERE organization_id = $1")
            .bind(fixture.organization.id.0)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(stored, 0);
}

#[tokio::test]
async fn without_secrets_key_or_console_url_the_endpoints_say_so_with_a_503() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let id = fixture.create("Mestier").await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let keyless = serve(
        &fixture.pool,
        &fixture.organization,
        Setup {
            keyed: false,
            ..USER
        },
    )
    .await;
    let headless = serve(
        &fixture.pool,
        &fixture.organization,
        Setup {
            console: false,
            ..USER
        },
    )
    .await;
    let base = format!("/api/v1/mcp-connectors/{id}");

    for (method, path, body) in [
        (Method::POST, format!("{base}/authorize"), json!({})),
        (Method::POST, format!("{base}/test"), json!({})),
        (
            Method::POST,
            "/api/v1/mcp-connectors/oauth/callback".to_owned(),
            json!({"state": "x", "code": "y"}),
        ),
    ] {
        let (status, response) = keyless.call(method, &path, Some(body)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path} {response}");
        assert_eq!(response["code"], "not_configured");
        assert!(
            response["message"]
                .as_str()
                .unwrap()
                .contains("SECRETS_KEY")
        );
    }
    let (status, response) = headless
        .post(
            "/api/v1/mcp-connectors",
            json!({"url": format!("{}/mcp", fixture.fake.base), "slug": "second"}),
        )
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        response["message"]
            .as_str()
            .unwrap()
            .contains("CONSOLE_URL")
    );
    let (status, _) = keyless.get("/api/v1/mcp-connectors").await;
    assert_eq!(status, StatusCode::OK, "reading needs no key");
}
