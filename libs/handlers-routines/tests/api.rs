#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, Actor, FerrisKeyVerifier, Identity};
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        crew::{
            CrewAgent,
            service::{CreateAgent, UpdateAgent},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::{AppState, Caller};
use chrono::{DateTime, Utc};
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
    usecase: CapitainerieUseCase,
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
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap())
        .with_secrets_cipher(SecretCipher::new(&[2u8; 32]).unwrap());
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
    let app: Router = capitainerie_handlers_routines::router(None)
        .layer(Extension(caller))
        .merge(capitainerie_handlers_routines::hooks_router())
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Some(Fixture {
        pool,
        usecase,
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

    async fn delete(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::DELETE, path, None).await
    }

    async fn hook(&self, token: &str) -> (StatusCode, Value) {
        let response = reqwest::Client::new()
            .post(format!("{}/hooks/routines/{token}", self.base))
            .body("{}")
            .send()
            .await
            .unwrap();
        let status = response.status();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    fn subject(&self) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: self.organization.id,
            actor: None,
            is_admin: false,
        }
    }

    async fn agent(&self, slug: &str) -> CrewAgent {
        self.usecase
            .create_crew_agent(
                self.subject(),
                CreateAgent {
                    slug: slug.into(),
                    name: slug.into(),
                    role: String::new(),
                    description: String::new(),
                    model: String::new(),
                    hue: 0,
                    reports_to: None,
                    instructions: "Tu aides.".into(),
                    connector_grants: vec![],
                },
            )
            .await
            .unwrap()
    }

    async fn create(&self, body: Value) -> Value {
        let (status, created) = self.post("/api/v1/routines", body).await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        created
    }
}

fn weekly() -> Value {
    json!({"frequency": "weekly", "time": "08:00", "days": ["mon", "thu"]})
}

#[tokio::test]
async fn a_routine_is_created_with_its_planning_and_read_back() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let agent = fixture.agent("archibald").await;
    let created = fixture
        .create(json!({
            "name": " Revue des accès externes du Drive ",
            "prompt": "Liste les partages externes.",
            "agent_id": agent.id,
            "priority": "high",
            "if_open": "queue",
            "if_missed": "run_all",
            "schedule": weekly(),
        }))
        .await;
    assert_eq!(created["name"], "Revue des accès externes du Drive");
    assert_eq!(created["slug"], "revue-des-acces-externes-du-drive");
    assert_eq!(created["prompt"], "Liste les partages externes.");
    assert_eq!(created["priority"], "high");
    assert_eq!(created["title_template"], "{nom} · {date}");
    assert_eq!(created["timezone"], "Europe/Paris");
    assert_eq!(created["enabled"], true);
    assert_eq!(created["if_open"], "queue");
    assert_eq!(created["if_missed"], "run_all");
    assert_eq!(created["allow_writes"], false);
    assert!(created["last_run"].is_null());
    assert_eq!(
        created["counts"],
        json!({"created": 0, "skipped": 0, "failed": 0})
    );
    let triggers = created["triggers"].as_array().unwrap();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0]["kind"], "planning");
    assert_eq!(triggers[0]["schedule"], weekly());
    let next: DateTime<Utc> = created["next_run_at"].as_str().unwrap().parse().unwrap();
    assert!(next > Utc::now());
    assert_eq!(created["next_run_at"], triggers[0]["next_run_at"]);

    let id = created["id"].as_str().unwrap();
    let (status, detail) = fixture.get(&format!("/api/v1/routines/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail, created);
    let (_, list) = fixture.get("/api/v1/routines").await;
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    assert_eq!(list["items"][0]["id"], id);
}

#[tokio::test]
async fn patching_pauses_moves_and_archives_the_routine() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let agent = fixture.agent("archibald").await;
    let other = fixture.agent("berenice").await;
    let created = fixture
        .create(json!({
            "name": "Revue",
            "prompt": "x",
            "agent_id": agent.id,
            "schedule": {"frequency": "daily", "time": "06:30"},
        }))
        .await;
    let path = format!("/api/v1/routines/{}", created["id"].as_str().unwrap());

    let (status, paused) = fixture
        .patch(
            &path,
            json!({"enabled": false, "agent_id": other.id, "name": "Revue du lundi", "if_open": "always"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{paused}");
    assert_eq!(paused["enabled"], false);
    assert!(paused["next_run_at"].is_null());
    assert!(paused["triggers"][0]["next_run_at"].is_null());
    assert_eq!(paused["agent_id"], json!(other.id));
    assert_eq!(paused["name"], "Revue du lundi");
    assert_eq!(paused["slug"], "revue");
    assert_eq!(paused["if_open"], "always");

    let (_, resumed) = fixture
        .patch(
            &path,
            json!({"enabled": true, "timezone": "America/New_York"}),
        )
        .await;
    assert_eq!(resumed["timezone"], "America/New_York");
    let next: DateTime<Utc> = resumed["next_run_at"].as_str().unwrap().parse().unwrap();
    let local = next.with_timezone(&chrono_tz::America::New_York);
    assert_eq!(local.format("%H:%M").to_string(), "06:30");

    let (status, _) = fixture.patch(&path, json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, archived) = fixture.post(&format!("{path}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(archived["archived"], true);
    assert!(archived["next_run_at"].is_null());
    let (status, _) = fixture.post(&format!("{path}/launch"), json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, active) = fixture.get("/api/v1/routines?archived=false").await;
    assert!(active["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_launch_creates_the_card_and_lists_the_passage() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let agent = fixture.agent("agathe").await;
    let created = fixture
        .create(json!({
            "name": "Newsletter",
            "prompt": "Rédige la newsletter.",
            "agent_id": agent.id,
            "title_template": "{nom} du {date}",
        }))
        .await;
    assert!(created["triggers"].as_array().unwrap().is_empty());
    assert!(created["next_run_at"].is_null());
    let path = format!("/api/v1/routines/{}", created["id"].as_str().unwrap());

    let (status, run) = fixture.post(&format!("{path}/launch"), json!({})).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{run}");
    assert_eq!(run["origin"], "manual");
    assert_eq!(run["status"], "created");
    assert!(run["run_id"].is_null());
    assert!(
        run["reason"]
            .as_str()
            .unwrap()
            .contains("exécution non lancée"),
        "{run}"
    );
    let card_id: Uuid = run["card_id"].as_str().unwrap().parse().unwrap();
    let (title, description, assignee): (String, String, Option<Uuid>) =
        sqlx::query_as("SELECT title, description, assignee_agent_id FROM cards WHERE id = $1")
            .bind(card_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert!(title.starts_with("Newsletter du "), "{title}");
    assert_eq!(description, "Rédige la newsletter.");
    assert_eq!(assignee, Some(agent.id.0));

    fixture.post(&format!("{path}/launch"), json!({})).await;
    let (status, runs) = fixture.get(&format!("{path}/runs?limit=1")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(runs["total"], 2);
    assert_eq!(runs["items"].as_array().unwrap().len(), 1);
    assert_eq!(runs["items"][0]["card_column"], "todo");
    let (_, detail) = fixture.get(&path).await;
    assert_eq!(detail["counts"]["created"], 2);
    assert_eq!(detail["last_run"]["status"], "created");
    assert!(detail["last_run_at"].is_string());
    let outbox: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM outbox_events WHERE organization_id = $1 AND topic = 'routines'",
    )
    .bind(fixture.organization.id.0)
    .fetch_all(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(outbox, ["routine.run_created", "routine.run_created"]);
}

#[tokio::test]
async fn a_webhook_trigger_runs_the_routine_and_follows_if_open() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let agent = fixture.agent("agathe").await;
    let created = fixture
        .create(json!({"name": "Relance", "prompt": "Relance.", "agent_id": agent.id}))
        .await;
    let path = format!("/api/v1/routines/{}", created["id"].as_str().unwrap());

    let (status, issued) = fixture
        .post(&format!("{path}/triggers"), json!({"kind": "webhook"}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{issued}");
    let token = issued["token"].as_str().unwrap().to_owned();
    assert!(token.starts_with("rtn_"));
    let url = issued["webhook_url"].as_str().unwrap();
    assert!(url.ends_with(&format!("/hooks/routines/{token}")), "{url}");
    let trigger = &issued["routine"]["triggers"][0];
    assert_eq!(trigger["kind"], "webhook");
    assert_eq!(trigger["id"], issued["trigger_id"]);
    assert!(token.ends_with(trigger["token_hint"].as_str().unwrap()));
    assert!(trigger["next_run_at"].is_null());
    let stored: String =
        sqlx::query_scalar("SELECT webhook_token_hash FROM routine_triggers WHERE id = $1")
            .bind(
                issued["trigger_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            )
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_ne!(stored, token);

    let (status, first) = fixture.hook(&token).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{first}");
    assert_eq!(first["origin"], "webhook");
    assert_eq!(first["status"], "created");
    let (status, second) = fixture.hook(&token).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(second["status"], "skipped", "{second}");
    assert!(second["card_id"].is_null());

    let trigger_path = format!("{path}/triggers/{}", issued["trigger_id"].as_str().unwrap());
    let (status, rotated) = fixture
        .post(&format!("{trigger_path}/rotate-token"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    let fresh = rotated["token"].as_str().unwrap();
    assert_ne!(fresh, token);
    let (status, _) = fixture.hook(&token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = fixture
        .patch(&trigger_path, json!({"enabled": false}))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = fixture.hook(fresh).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, removed) = fixture.delete(&trigger_path).await;
    assert_eq!(status, StatusCode::OK);
    assert!(removed["triggers"].as_array().unwrap().is_empty());
    let (status, _) = fixture.delete(&trigger_path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for bad in ["nope", "rtn_unknown"] {
        let (status, _) = fixture.hook(bad).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{bad}");
    }
}

#[tokio::test]
async fn a_planning_trigger_is_added_patched_and_validated() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let agent = fixture.agent("agathe").await;
    let created = fixture
        .create(json!({"name": "SEO", "prompt": "x", "agent_id": agent.id}))
        .await;
    let path = format!("/api/v1/routines/{}", created["id"].as_str().unwrap());

    let (status, issued) = fixture
        .post(
            &format!("{path}/triggers"),
            json!({"kind": "planning", "schedule": {"frequency": "every_hours", "hours": 48}}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{issued}");
    assert!(issued["token"].is_null());
    let trigger_path = format!("{path}/triggers/{}", issued["trigger_id"].as_str().unwrap());
    let (status, patched) = fixture
        .patch(
            &trigger_path,
            json!({"schedule": {"frequency": "cron", "cron": "30 6 * * 1-5"}}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_eq!(
        patched["triggers"][0]["schedule"],
        json!({"frequency": "cron", "cron": "30 6 * * 1-5"})
    );
    assert!(patched["triggers"][0]["next_run_at"].is_string());

    for invalid in [
        json!({"kind": "planning"}),
        json!({"kind": "webhook", "schedule": weekly()}),
        json!({"kind": "planning", "schedule": {"frequency": "cron", "cron": "61 * * * *"}}),
        json!({"kind": "planning", "schedule": {"frequency": "weekly", "days": []}}),
        json!({"kind": "planning", "schedule": {"frequency": "daily", "time": "25:00"}}),
        json!({"kind": "rss"}),
    ] {
        let (status, body) = fixture.post(&format!("{path}/triggers"), invalid).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
    let (status, _) = fixture
        .post(&format!("{trigger_path}/rotate-token"), json!({}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn invalid_input_and_a_duplicate_slug_are_refused() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let agent = fixture.agent("agathe").await;
    let archived = fixture.agent("ancien").await;
    fixture
        .usecase
        .update_crew_agent(
            fixture.subject(),
            archived.id,
            UpdateAgent {
                archived: Some(true),
                ..UpdateAgent::default()
            },
        )
        .await
        .unwrap();
    fixture
        .create(json!({"slug": "revue", "name": "Revue", "prompt": "x", "agent_id": agent.id}))
        .await;
    let (status, _) = fixture
        .post(
            "/api/v1/routines",
            json!({"slug": "revue", "name": "Autre", "prompt": "x", "agent_id": agent.id}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let base = json!({"name": "x", "prompt": "x", "agent_id": agent.id});
    let with = |field: &str, value: Value| {
        let mut body = base.clone();
        body[field] = value;
        body
    };
    for invalid in [
        with("name", json!(" ")),
        with("prompt", json!("")),
        with("prompt", json!("x".repeat(20_001))),
        with("slug", json!("Not Kebab")),
        with("agent_id", json!(Uuid::now_v7())),
        with("agent_id", json!(archived.id)),
        with("project_id", json!(Uuid::now_v7())),
        with("timezone", json!("Mars/Olympus")),
        with("if_open", json!("stack")),
        with("if_missed", json!("never")),
        with("priority", json!("asap")),
        with("title_template", json!(" ")),
        with("schedule", json!({"frequency": "hourly"})),
        json!({"name": "x", "prompt": "x"}),
    ] {
        let (status, body) = fixture.post("/api/v1/routines", invalid.clone()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{invalid} {body}");
    }
    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM routines WHERE organization_id = $1")
            .bind(fixture.organization.id.0)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(stored, 1);
}

#[tokio::test]
async fn a_routine_of_another_organization_is_a_404() {
    let Some(mine) = fixture(None).await else {
        return;
    };
    let Some(theirs) = fixture(None).await else {
        return;
    };
    let agent = theirs.agent("agathe").await;
    let created = theirs
        .create(json!({
            "name": "Revue",
            "prompt": "x",
            "agent_id": agent.id,
            "schedule": weekly(),
        }))
        .await;
    let path = format!("/api/v1/routines/{}", created["id"].as_str().unwrap());
    let trigger_path = format!(
        "{path}/triggers/{}",
        created["triggers"][0]["id"].as_str().unwrap()
    );

    let (status, _) = mine.get(&path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine.patch(&path, json!({"name": "Pris"})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for action in ["archive", "launch"] {
        let (status, _) = mine.post(&format!("{path}/{action}"), json!({})).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{action}");
    }
    let (status, _) = mine.get(&format!("{path}/runs")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine
        .post(&format!("{path}/triggers"), json!({"kind": "webhook"}))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine.patch(&trigger_path, json!({"enabled": false})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine.delete(&trigger_path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, untouched) = theirs.get(&path).await;
    assert_eq!(untouched["name"], "Revue");
    assert_eq!(untouched["triggers"][0]["enabled"], true);
    assert_eq!(untouched["counts"]["created"], 0);
}

#[tokio::test]
async fn an_agent_caller_cannot_read_or_manage_routines() {
    let Some(fixture) = fixture(Some("capitainerie-agent-writer")).await else {
        return;
    };
    let (status, _) = fixture.get("/api/v1/routines").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture
        .post(
            "/api/v1/routines",
            json!({"name": "Revue", "prompt": "x", "agent_id": Uuid::now_v7()}),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM routines WHERE organization_id = $1")
            .bind(fixture.organization.id.0)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(stored, 0);
}
