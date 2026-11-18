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
use reqwest::StatusCode;
use serde_json::Value;
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
    let app: Router = capitainerie_handlers_artifacts::router()
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
    async fn get(&self, path: &str) -> (StatusCode, Value) {
        let response = reqwest::get(format!("{}{path}", self.base)).await.unwrap();
        let status = response.status();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    async fn agent(&self, slug: &str, name: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO agents (id, organization_id, slug, name, created_by, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, now(), now())",
        )
        .bind(id)
        .bind(self.organization.id.0)
        .bind(slug)
        .bind(name)
        .bind(USER)
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }

    async fn card(&self, title: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query(
            r#"INSERT INTO cards (id, organization_id, title, description, "column", version, created_by_subject, created_at, updated_at) VALUES ($1, $2, $3, '', 'todo', 1, $4, now(), now())"#,
        )
        .bind(id)
        .bind(self.organization.id.0)
        .bind(title)
        .bind(USER)
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }

    async fn artifact(
        &self,
        title: &str,
        kind: &str,
        agent: Option<Uuid>,
        card: Option<Uuid>,
    ) -> Uuid {
        let id = Uuid::now_v7();
        let (content, url) = match kind {
            "link" => (None, Some("https://example.com/plan")),
            _ => (Some(format!("# {title}")), None),
        };
        sqlx::query(
            "INSERT INTO artifacts (id, organization_id, card_id, agent_id, kind, title, content, url, created_by, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now(), now())",
        )
        .bind(id)
        .bind(self.organization.id.0)
        .bind(card)
        .bind(agent)
        .bind(kind)
        .bind(title)
        .bind(content)
        .bind(url)
        .bind(USER)
        .execute(&self.pool)
        .await
        .unwrap();
        id
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
async fn artifacts_are_listed_with_their_agent_filtered_and_paged() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let constance = fixture.agent("constance", "Constance (Marketing)").await;
    let henri = fixture.agent("henri", "Henri").await;
    let card = fixture.card("Plan S47").await;
    fixture
        .artifact(
            "Plan de la semaine 47",
            "markdown",
            Some(constance),
            Some(card),
        )
        .await;
    fixture
        .artifact("Latence des appels", "json", Some(henri), None)
        .await;
    fixture.artifact("Schéma", "link", None, None).await;

    let (status, list) = fixture.get("/api/v1/artifacts").await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(
        titles(&list),
        ["Schéma", "Latence des appels", "Plan de la semaine 47"]
    );
    let plan = &list["items"][2];
    assert_eq!(plan["kind"], "markdown");
    assert_eq!(plan["agent_name"], "Constance (Marketing)");
    assert_eq!(plan["card_title"], "Plan S47");
    assert!(plan.get("content").is_none());
    assert_eq!(list["items"][0]["url"], "https://example.com/plan");

    let (_, by_agent) = fixture
        .get(&format!("/api/v1/artifacts?agent_id={henri}"))
        .await;
    assert_eq!(titles(&by_agent), ["Latence des appels"]);
    let (_, by_card) = fixture
        .get(&format!("/api/v1/artifacts?card_id={card}"))
        .await;
    assert_eq!(titles(&by_card), ["Plan de la semaine 47"]);
    let (_, links) = fixture.get("/api/v1/artifacts?kind=link").await;
    assert_eq!(titles(&links), ["Schéma"]);
    let (status, _) = fixture.get("/api/v1/artifacts?kind=pdf").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (_, first) = fixture.get("/api/v1/artifacts?limit=2").await;
    assert_eq!(titles(&first).len(), 2);
    let cursor = first["next_cursor"].as_str().unwrap();
    let (_, rest) = fixture
        .get(&format!("/api/v1/artifacts?limit=2&cursor={cursor}"))
        .await;
    assert_eq!(titles(&rest), ["Plan de la semaine 47"]);
    assert_eq!(rest["next_cursor"], Value::Null);
}

#[tokio::test]
async fn an_artifact_is_read_with_its_content() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let agent = fixture.agent("iris", "Iris").await;
    let id = fixture
        .artifact("Rapport", "markdown", Some(agent), None)
        .await;
    let (status, artifact) = fixture.get(&format!("/api/v1/artifacts/{id}")).await;
    assert_eq!(status, StatusCode::OK, "{artifact}");
    assert_eq!(artifact["content"], "# Rapport");
    assert_eq!(artifact["agent_name"], "Iris");
    assert_eq!(artifact["created_by"], USER);
    let (status, _) = fixture
        .get(&format!("/api/v1/artifacts/{}", Uuid::now_v7()))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn another_organization_s_artifacts_are_invisible() {
    let Some(mine) = fixture(None).await else {
        return;
    };
    let Some(theirs) = fixture(None).await else {
        return;
    };
    let foreign = theirs.artifact("Ailleurs", "markdown", None, None).await;
    let (_, list) = mine.get("/api/v1/artifacts").await;
    assert!(titles(&list).is_empty());
    let (status, _) = mine.get(&format!("/api/v1/artifacts/{foreign}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_agent_reads_no_artifact() {
    let Some(fixture) = fixture(Some("agent:iris")).await else {
        return;
    };
    let id = fixture.artifact("Rapport", "markdown", None, None).await;
    let (status, _) = fixture.get("/api/v1/artifacts").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture.get(&format!("/api/v1/artifacts/{id}")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
