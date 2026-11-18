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

const USER: &str = "user-1";

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

async fn fixture(actor: Option<&str>) -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 3).await.unwrap();
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = format!("http://idp.test/realms/{slug}");
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(&slug)
    .bind(&issuer_url)
    .execute(&pool)
    .await
    .unwrap();
    let organization = Organization {
        id: OrganizationId(id),
        slug: OrganizationSlug::try_from(slug).unwrap(),
        name: "Test".into(),
        issuer_url,
        created_at: Utc::now(),
    };
    let state = AppState {
        usecase: CapitainerieUseCase::with_local_policies(pool.clone()),
        verifier: FerrisKeyVerifier::new(AcceptedClients::new(["capitainerie-console"])),
        delegation: None,
    };
    let caller = Caller {
        identity: Identity {
            subject: USER.into(),
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
    let app: Router = capitainerie_handlers_inbox::router()
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

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::GET, path, None).await
    }

    async fn read(&self, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, "/api/v1/inbox/read", Some(body))
            .await
    }

    async fn item(&self, member: &str, title: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO inbox_items (id, organization_id, member_subject, event_id, kind, title, body, link, created_at) VALUES ($1, $2, $3, $4, 'run', $5, 'détail', '/tasks/x', now())",
        )
        .bind(id)
        .bind(self.organization.id.0)
        .bind(member)
        .bind(Uuid::now_v7())
        .bind(title)
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }

    async fn read_at(&self, id: Uuid) -> Option<chrono::DateTime<Utc>> {
        sqlx::query_scalar("SELECT read_at FROM inbox_items WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

fn titles(list: &Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["title"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn a_member_lists_their_own_items_newest_first_and_pages() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    fixture.item(USER, "Premier").await;
    fixture.item("user-2", "Pas pour moi").await;
    fixture.item(USER, "Deuxième").await;
    fixture.item(USER, "Troisième").await;

    let (status, list) = fixture.get("/api/v1/inbox").await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(titles(&list), ["Troisième", "Deuxième", "Premier"]);
    assert_eq!(
        (list["total"].as_i64(), list["unread"].as_i64()),
        (Some(3), Some(3))
    );
    let item = &list["items"][0];
    assert_eq!(item["kind"], "run");
    assert_eq!(item["body"], "détail");
    assert_eq!(item["link"], "/tasks/x");
    assert_eq!(item["unread"], true);
    assert_eq!(item["read_at"], Value::Null);
    assert_eq!(list["next_cursor"], Value::Null);

    let (_, first) = fixture.get("/api/v1/inbox?limit=2").await;
    assert_eq!(titles(&first), ["Troisième", "Deuxième"]);
    let cursor = first["next_cursor"].as_str().unwrap();
    let (_, second) = fixture
        .get(&format!("/api/v1/inbox?limit=2&cursor={cursor}"))
        .await;
    assert_eq!(titles(&second), ["Premier"]);
    assert_eq!(second["next_cursor"], Value::Null);

    let (status, _) = fixture.get("/api/v1/inbox?limit=500").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn items_are_marked_read_by_id_or_all_at_once() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let first = fixture.item(USER, "Premier").await;
    let second = fixture.item(USER, "Deuxième").await;
    fixture.item(USER, "Troisième").await;
    let foreign = fixture.item("user-2", "Pas pour moi").await;

    let (status, marked) = fixture.read(json!({"ids": [first, foreign]})).await;
    assert_eq!(status, StatusCode::OK, "{marked}");
    assert_eq!(marked, json!({"updated": 1, "unread": 2}));
    assert!(fixture.read_at(first).await.is_some());
    assert!(fixture.read_at(foreign).await.is_none());

    let (_, unread) = fixture.get("/api/v1/inbox?unread_only=true").await;
    assert_eq!(titles(&unread), ["Troisième", "Deuxième"]);
    assert_eq!(unread["total"], 3);

    let (_, again) = fixture.read(json!({"ids": [first]})).await;
    assert_eq!(again["updated"], 0);

    let (_, all) = fixture.read(json!({"all": true})).await;
    assert_eq!(all, json!({"updated": 2, "unread": 0}));
    assert!(fixture.read_at(second).await.is_some());
    assert!(fixture.read_at(foreign).await.is_none());
    let (_, list) = fixture.get("/api/v1/inbox").await;
    assert_eq!(list["items"][0]["unread"], false);
    assert!(list["items"][0]["read_at"].is_string());

    for body in [
        json!({}),
        json!({"ids": []}),
        json!({"ids": [first], "all": true}),
    ] {
        let (status, _) = fixture.read(body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn another_organization_s_items_are_neither_listed_nor_marked() {
    let Some(mine) = fixture(None).await else {
        return;
    };
    let Some(theirs) = fixture(None).await else {
        return;
    };
    let foreign = theirs.item(USER, "Ailleurs").await;
    let (_, list) = mine.get("/api/v1/inbox").await;
    assert!(titles(&list).is_empty());
    assert_eq!(list["total"], 0);
    let (_, marked) = mine.read(json!({"ids": [foreign]})).await;
    assert_eq!(marked["updated"], 0);
    let (_, all) = mine.read(json!({"all": true})).await;
    assert_eq!(all["updated"], 0);
    assert!(theirs.read_at(foreign).await.is_none());
}

#[tokio::test]
async fn an_agent_has_no_inbox() {
    let Some(fixture) = fixture(Some("agent:iris")).await else {
        return;
    };
    let id = fixture.item(USER, "Secret").await;
    let (status, _) = fixture.get("/api/v1/inbox").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture.read(json!({"all": true})).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(fixture.read_at(id).await.is_none());
}
