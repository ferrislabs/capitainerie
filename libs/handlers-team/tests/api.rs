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
    let app: Router = capitainerie_handlers_team::router()
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

#[tokio::test]
async fn an_agent_is_created_read_updated_and_archived() {
    let Some(fixture) = fixture(None).await else {
        return;
    };

    let (status, created) = fixture
        .post(
            "/api/v1/team/agents",
            json!({
                "slug": "dev",
                "name": " Dev ",
                "role": "Développeur",
                "model": "claude-sonnet",
                "hue": 210,
                "instructions": "Tu codes.",
                "connector_grants": ["github", "github", "odoo"],
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["name"], "Dev");
    assert_eq!(created["slug"], "dev");
    assert_eq!(created["status"], "idle");
    assert_eq!(created["archived"], false);
    assert_eq!(created["reports_to"], Value::Null);
    assert_eq!(created["connector_grants"], json!(["github", "odoo"]));
    assert_eq!(created["created_by"], "user-1");
    let path = format!("/api/v1/team/agents/{}", created["id"].as_str().unwrap());

    let (_, list) = fixture.get("/api/v1/team/agents").await;
    assert_eq!(slugs(&list), ["dev"]);
    let (status, detail) = fixture.get(&path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["instructions"], "Tu codes.");
    assert_eq!(detail["hue"], 210);

    let (status, updated) = fixture
        .patch(
            &path,
            json!({"name": "Développeuse", "connector_grants": [], "hue": 30}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["name"], "Développeuse");
    assert_eq!(updated["connector_grants"], json!([]));
    assert_eq!(updated["role"], "Développeur");
    assert_eq!(updated["hue"], 30);

    let (status, archived) = fixture.patch(&path, json!({"archived": true})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(archived["archived"], true);
    let (_, active) = fixture.get("/api/v1/team/agents").await;
    assert!(slugs(&active).is_empty());
    let (_, only_archived) = fixture.get("/api/v1/team/agents?archived=true").await;
    assert_eq!(slugs(&only_archived), ["dev"]);
    let (status, _) = fixture.call(Method::DELETE, &path, None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn an_empty_model_is_accepted_on_create_and_clears_on_patch() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let (status, created) = fixture
        .post(
            "/api/v1/team/agents",
            json!({"slug": "dev", "name": "Dev", "model": ""}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["model"], "");
    let path = format!("/api/v1/team/agents/{}", created["id"].as_str().unwrap());

    let (status, set) = fixture
        .patch(&path, json!({"model": "claude-sonnet-5-5"}))
        .await;
    assert_eq!(status, StatusCode::OK, "{set}");
    assert_eq!(set["model"], "claude-sonnet-5-5");
    let (status, cleared) = fixture.patch(&path, json!({"model": ""})).await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["model"], "");
}

#[tokio::test]
async fn the_reporting_line_is_set_cleared_and_never_loops() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let (_, boss) = fixture
        .post(
            "/api/v1/team/agents",
            json!({"slug": "boss", "name": "Boss"}),
        )
        .await;
    let (status, lead) = fixture
        .post(
            "/api/v1/team/agents",
            json!({"slug": "lead", "name": "Lead", "reports_to": boss["id"]}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{lead}");
    assert_eq!(lead["reports_to"], boss["id"]);
    let boss_path = format!("/api/v1/team/agents/{}", boss["id"].as_str().unwrap());
    let lead_path = format!("/api/v1/team/agents/{}", lead["id"].as_str().unwrap());

    let (status, _) = fixture
        .patch(&boss_path, json!({"reports_to": lead["id"]}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .patch(&boss_path, json!({"reports_to": boss["id"]}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .patch(&lead_path, json!({"reports_to": Uuid::now_v7()}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, kept) = fixture.patch(&lead_path, json!({"role": "Lead"})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(kept["reports_to"], boss["id"]);
    let (status, cleared) = fixture.patch(&lead_path, json!({"reports_to": null})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cleared["reports_to"], Value::Null);
}

#[tokio::test]
async fn invalid_input_a_duplicate_slug_and_an_empty_patch_are_refused() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let (status, created) = fixture
        .post("/api/v1/team/agents", json!({"slug": "dev", "name": "Dev"}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let path = format!("/api/v1/team/agents/{}", created["id"].as_str().unwrap());

    let (status, _) = fixture
        .post(
            "/api/v1/team/agents",
            json!({"slug": "dev", "name": "Autre"}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    for body in [
        json!({"slug": "Not Kebab", "name": "x"}),
        json!({"slug": "ok", "name": " "}),
        json!({"slug": "ok", "name": "x", "hue": 400}),
        json!({"slug": "ok", "name": "x", "connector_grants": ["a,b"]}),
    ] {
        let (status, _) = fixture.post("/api/v1/team/agents", body.clone()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
    let (status, _) = fixture.patch(&path, json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, list) = fixture.get("/api/v1/team/agents").await;
    assert_eq!(slugs(&list), ["dev"]);
}

#[tokio::test]
async fn an_agent_of_another_organization_is_a_404_not_a_403() {
    let Some(mine) = fixture(None).await else {
        return;
    };
    let Some(theirs) = fixture(None).await else {
        return;
    };
    let (_, foreign) = theirs
        .post("/api/v1/team/agents", json!({"slug": "dev", "name": "Dev"}))
        .await;
    let path = format!("/api/v1/team/agents/{}", foreign["id"].as_str().unwrap());

    let (status, _) = mine.get(&path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine.patch(&path, json!({"name": "Pris"})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine
        .post(
            "/api/v1/team/agents",
            json!({"slug": "mine", "name": "Mine", "reports_to": foreign["id"]}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, list) = mine.get("/api/v1/team/agents").await;
    assert!(slugs(&list).is_empty());
    let (_, untouched) = theirs.get(&path).await;
    assert_eq!(untouched["name"], "Dev");
}

#[tokio::test]
async fn an_agent_caller_cannot_read_or_manage_the_registry() {
    let Some(fixture) = fixture(Some("capitainerie-agent-writer")).await else {
        return;
    };

    let (status, _) = fixture.get("/api/v1/team/agents").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture
        .post("/api/v1/team/agents", json!({"slug": "dev", "name": "Dev"}))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM agents WHERE organization_id = $1")
        .bind(fixture.organization.id.0)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}
