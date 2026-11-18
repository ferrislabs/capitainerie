#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, Actor, FerrisKeyVerifier, Identity};
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug, domain::agent::SecretToken,
    infrastructure::postgres,
};
use capitainerie_handlers::{AppState, Caller};
use chrono::Utc;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

struct Fixture {
    pool: PgPool,
    organization: Organization,
    base: String,
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

async fn fixture(actor: Option<&str>) -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 3).await.unwrap();
    let organization = organization(&pool).await;
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone());
    let state = AppState {
        usecase: usecase.clone(),
        verifier: FerrisKeyVerifier::new(AcceptedClients::new(["capitainerie-console"])),
        delegation: None,
    };
    let caller = Caller {
        identity: Identity {
            subject: "user-1".into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: actor.map(|client_id| Actor {
                sub: "service-account".into(),
                client_id: Some(client_id.to_owned()),
                act: None,
            }),
            roles: vec![],
        },
        organization: organization.clone(),
        token: SecretToken::new("user-token"),
    };
    let app: Router = capitainerie_handlers_skills::router()
        .layer(Extension(caller))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Some(Fixture {
        pool,
        organization,
        base: format!("http://{address}"),
    })
}

impl Fixture {
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

    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, path, Some(body)).await
    }

    async fn patch(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::PATCH, path, Some(body)).await
    }

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::GET, path, None).await
    }
}

fn slugs(list: &Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["slug"].as_str().unwrap())
        .collect()
}

impl Fixture {
    async fn agent(&self, slug: &str, skills: &[&str], archived: bool) {
        sqlx::query(
            "INSERT INTO agents (id, organization_id, slug, name, skills, archived, created_by, created_at, updated_at)
             VALUES ($1, $2, $3, $3, $4, $5, 'user-1', now(), now())",
        )
        .bind(Uuid::now_v7())
        .bind(self.organization.id.0)
        .bind(slug)
        .bind(skills)
        .bind(archived)
        .execute(&self.pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn a_skill_is_created_read_updated_and_archived() {
    let Some(fixture) = fixture(None).await else {
        return;
    };

    let (status, created) = fixture
        .post(
            "/api/v1/skills",
            json!({
                "slug": "humanizer",
                "name": " humanizer ",
                "description": "Retire les marques d'écriture IA.",
                "content": "# Humanizer\nÉcris simplement.",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["name"], "humanizer");
    assert_eq!(created["source"], "manual");
    assert_eq!(created["archived"], false);
    assert_eq!(created["used_by"], 0);
    assert_eq!(created["created_by"], "user-1");
    let path = format!("/api/v1/skills/{}", created["id"].as_str().unwrap());

    let (status, detail) = fixture.get(&path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["content"], "# Humanizer\nÉcris simplement.");

    let (status, updated) = fixture
        .patch(
            &path,
            json!({"content": "Écris court.", "source": "https://example.com/humanizer.md"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["content"], "Écris court.");
    assert_eq!(updated["source"], "https://example.com/humanizer.md");
    assert_eq!(updated["description"], "Retire les marques d'écriture IA.");

    let (status, archived) = fixture.post(&format!("{path}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{archived}");
    assert_eq!(archived["archived"], true);
    let (status, again) = fixture.post(&format!("{path}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["archived"], true);
    let (_, archived_only) = fixture.get("/api/v1/skills?archived=true").await;
    assert_eq!(slugs(&archived_only), ["humanizer"]);
    let (_, active) = fixture.get("/api/v1/skills?archived=false").await;
    assert!(slugs(&active).is_empty());

    let (status, restored) = fixture.patch(&path, json!({"archived": false})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(restored["archived"], false);
    let (status, _) = fixture.call(Method::DELETE, &path, None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn used_by_counts_the_active_agents_naming_the_slug() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    fixture.agent("dev", &["review", "deploy"], false).await;
    fixture.agent("lead", &["review"], false).await;
    fixture.agent("old", &["review"], true).await;
    fixture.agent("ops", &["deploy"], false).await;
    for slug in ["review", "deploy", "unused"] {
        let (status, body) = fixture
            .post("/api/v1/skills", json!({"slug": slug, "name": slug}))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }

    let (status, list) = fixture.get("/api/v1/skills").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(slugs(&list), ["deploy", "review", "unused"]);
    let used_by: Vec<i64> = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["used_by"].as_i64().unwrap())
        .collect();
    assert_eq!(used_by, [2, 2, 0]);
}

#[tokio::test]
async fn invalid_input_a_duplicate_slug_and_an_empty_patch_are_refused() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let (status, created) = fixture
        .post(
            "/api/v1/skills",
            json!({"slug": "review", "name": "Relecture"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let path = format!("/api/v1/skills/{}", created["id"].as_str().unwrap());

    let (status, _) = fixture
        .post("/api/v1/skills", json!({"slug": "review", "name": "Autre"}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    for invalid in [
        json!({"slug": "Not Kebab", "name": "x"}),
        json!({"slug": "ok", "name": " "}),
        json!({"slug": "ok", "name": "x", "source": "ftp://example.com"}),
        json!({"slug": "ok", "name": "x", "content": "a".repeat(100_001)}),
    ] {
        let (status, body) = fixture.post("/api/v1/skills", invalid).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
    let (status, _) = fixture.patch(&path, json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, list) = fixture.get("/api/v1/skills?archived=false").await;
    assert_eq!(slugs(&list), ["review"]);
}

#[tokio::test]
async fn a_skill_of_another_organization_is_a_404() {
    let Some(mine) = fixture(None).await else {
        return;
    };
    let Some(theirs) = fixture(None).await else {
        return;
    };
    let (_, foreign) = theirs
        .post(
            "/api/v1/skills",
            json!({"slug": "review", "name": "Relecture"}),
        )
        .await;
    let path = format!("/api/v1/skills/{}", foreign["id"].as_str().unwrap());

    let (status, _) = mine.get(&path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine.patch(&path, json!({"name": "Pris"})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine.post(&format!("{path}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, untouched) = theirs.get(&path).await;
    assert_eq!(untouched["name"], "Relecture");
    assert_eq!(untouched["archived"], false);
}

#[tokio::test]
async fn an_agent_caller_cannot_read_or_manage_the_catalog() {
    let Some(fixture) = fixture(Some("capitainerie-agent-writer")).await else {
        return;
    };
    let (status, _) = fixture.get("/api/v1/skills").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture
        .post(
            "/api/v1/skills",
            json!({"slug": "review", "name": "Relecture"}),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM skills WHERE organization_id = $1")
        .bind(fixture.organization.id.0)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}
