#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, Actor, FerrisKeyVerifier, Identity};
use capitainerie_common::{AgentId, CardId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken, authorization::Subject, card::service::CreateCard,
        crew::service::CreateAgent,
    },
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

#[derive(Clone, Copy)]
enum Who {
    Member,
    Admin,
    Agent,
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
    sqlx::query(
        "INSERT INTO members (organization_id, subject, name, last_seen_at) VALUES ($1, $2, 'Alice Martin', now())",
    )
    .bind(id)
    .bind(USER)
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

async fn fixture(who: Who) -> Option<Fixture> {
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
            subject: USER.into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: matches!(who, Who::Agent).then(|| Actor {
                sub: "service-account".into(),
                client_id: Some("agent:iris".into()),
                act: None,
            }),
            roles: match who {
                Who::Member => vec![],
                Who::Admin | Who::Agent => vec!["capitainerie-admin".into()],
            },
        },
        organization: organization.clone(),
        token: SecretToken::new("user-token"),
    };
    let app: Router = capitainerie_handlers_audit::router()
        .layer(Extension(caller))
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

/// What an agent task left behind: a card, its run, a comment by the agent
/// and a step that cost `cost_micros`.
struct Story {
    agent_id: AgentId,
    card_id: CardId,
    run_id: Uuid,
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

    async fn put(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::PUT, path, Some(body)).await
    }

    fn human(&self) -> Subject {
        Subject {
            id: USER.into(),
            organization_id: self.organization.id,
            actor: None,
            is_admin: false,
        }
    }

    async fn agent(&self, slug: &str, name: &str) -> AgentId {
        self.usecase
            .create_crew_agent(
                self.human(),
                CreateAgent {
                    slug: slug.into(),
                    name: name.into(),
                    role: String::new(),
                    description: String::new(),
                    model: String::new(),
                    hue: 0,
                    reports_to: None,
                    instructions: String::new(),
                    connector_grants: vec![],
                    repository_grants: vec![],
                },
            )
            .await
            .unwrap()
            .id
    }

    async fn story(&self, cost_micros: i64) -> Story {
        let agent_id = self.agent("iris", "Iris").await;
        let card = self
            .usecase
            .create_card_linked(
                self.human(),
                CreateCard {
                    title: "Relancer les factures".into(),
                    description: String::new(),
                    provenance: None,
                    run_id: None,
                    attributes: Default::default(),
                },
                None,
                None,
            )
            .await
            .unwrap();
        let run_id = Uuid::now_v7();
        let organization_id = self.organization.id.0;
        sqlx::query(
            "INSERT INTO runs (id, organization_id, card_id, initiated_by, status, pipeline, created_at) VALUES ($1, $2, $3, $4, 'running', '{agent_task}', now())",
        )
        .bind(run_id)
        .bind(organization_id)
        .bind(card.id.0)
        .bind(USER)
        .execute(&self.pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO agent_tasks (run_id, organization_id, card_id, agent_id, allow_writes, created_by, created_at) VALUES ($1, $2, $3, $4, false, $5, now())",
        )
        .bind(run_id)
        .bind(organization_id)
        .bind(card.id.0)
        .bind(agent_id.0)
        .bind(USER)
        .execute(&self.pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO run_steps (id, run_id, organization_id, seq, kind, status, attempt, cost_micros, started_at) VALUES ($1, $2, $3, 0, 'agent_task', 'succeeded', 1, $4, now())",
        )
        .bind(Uuid::now_v7())
        .bind(run_id)
        .bind(organization_id)
        .bind(cost_micros)
        .execute(&self.pool)
        .await
        .unwrap();
        self.usecase
            .comment_card_as_agent(
                capitainerie_common::RunId(run_id),
                card.id,
                "iris",
                "Trois factures en retard.".into(),
            )
            .await
            .unwrap();
        Story {
            agent_id,
            card_id: card.id,
            run_id,
        }
    }
}

fn kinds(list: &Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["type"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn the_log_names_members_and_agents_and_summarizes_each_event() {
    let Some(fixture) = fixture(Who::Member).await else {
        return;
    };
    let story = fixture.story(0).await;

    let (status, list) = fixture.get("/api/v1/audit/events").await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(kinds(&list), ["card.commented", "card.proposed"]);
    let comment = &list["items"][0];
    assert_eq!(comment["actor"]["kind"], "agent");
    assert_eq!(comment["actor"]["name"], "Iris");
    assert_eq!(comment["actor"]["id"], story.agent_id.0.to_string());
    assert_eq!(
        comment["summary"],
        "a commenté la carte « Relancer les factures »"
    );
    assert_eq!(comment["severity"], "info");
    assert_eq!(comment["topic"], "cards");
    assert_eq!(comment["card_id"], story.card_id.0.to_string());
    assert_eq!(comment["run_id"], story.run_id.to_string());
    let proposed = &list["items"][1];
    assert_eq!(proposed["actor"]["kind"], "member");
    assert_eq!(proposed["actor"]["name"], "Alice Martin");
    assert_eq!(proposed["actor"]["id"], USER);
    assert_eq!(proposed["severity"], "created");
    assert_eq!(list["next_cursor"], Value::Null);
}

#[tokio::test]
async fn the_log_filters_and_pages_with_a_cursor() {
    let Some(fixture) = fixture(Who::Member).await else {
        return;
    };
    let story = fixture.story(0).await;
    let card = story.card_id.0;

    let (_, first) = fixture.get("/api/v1/audit/events?limit=1").await;
    assert_eq!(kinds(&first), ["card.commented"]);
    let cursor = first["next_cursor"].as_str().unwrap();
    let (_, second) = fixture
        .get(&format!("/api/v1/audit/events?limit=1&cursor={cursor}"))
        .await;
    assert_eq!(kinds(&second), ["card.proposed"]);
    assert_eq!(second["next_cursor"], Value::Null);

    let (_, typed) = fixture.get("/api/v1/audit/events?type=card.proposed").await;
    assert_eq!(kinds(&typed), ["card.proposed"]);
    let (_, by_agent) = fixture
        .get(&format!(
            "/api/v1/audit/events?agent_id={}",
            story.agent_id.0
        ))
        .await;
    assert_eq!(kinds(&by_agent), ["card.commented"]);
    let (_, by_card) = fixture
        .get(&format!(
            "/api/v1/audit/events?card_id={card}&topic=cards&actor={USER}"
        ))
        .await;
    assert_eq!(kinds(&by_card), ["card.commented", "card.proposed"]);
    let (_, nobody) = fixture.get("/api/v1/audit/events?actor=user-9").await;
    assert!(kinds(&nobody).is_empty());
    let (_, other_card) = fixture
        .get(&format!("/api/v1/audit/events?card_id={}", Uuid::now_v7()))
        .await;
    assert!(kinds(&other_card).is_empty());
    let future = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    let (_, later) = fixture
        .call(
            Method::GET,
            &format!("/api/v1/audit/events?from={}", urlencode(&future)),
            None,
        )
        .await;
    assert!(kinds(&later).is_empty());

    let (status, _) = fixture.get("/api/v1/audit/events?limit=0").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture.get("/api/v1/audit/events?cursor=nope").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

fn urlencode(value: &str) -> String {
    value.replace('+', "%2B").replace(':', "%3A")
}

#[tokio::test]
async fn another_organization_sees_none_of_the_events() {
    let Some(mine) = fixture(Who::Member).await else {
        return;
    };
    let Some(theirs) = fixture(Who::Member).await else {
        return;
    };
    let story = theirs.story(1_000_000).await;
    let (_, list) = mine.get("/api/v1/audit/events").await;
    assert!(kinds(&list).is_empty());
    let (_, by_card) = mine
        .get(&format!("/api/v1/audit/events?card_id={}", story.card_id.0))
        .await;
    assert!(kinds(&by_card).is_empty());
    let (_, budgets) = mine.get("/api/v1/budgets").await;
    assert_eq!(budgets["spent_micros"], 0);
}

#[tokio::test]
async fn an_agent_reads_no_log_and_no_budget() {
    let Some(fixture) = fixture(Who::Agent).await else {
        return;
    };
    let (status, _) = fixture.get("/api/v1/audit/events").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture.get("/api/v1/budgets").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture
        .put(
            "/api/v1/budgets",
            json!({"organization": {"monthly_limit_cents": 100}}),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM budgets WHERE organization_id = $1")
        .bind(fixture.organization.id.0)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn an_admin_sets_budgets_and_the_month_spend_is_measured_against_them() {
    let Some(fixture) = fixture(Who::Admin).await else {
        return;
    };
    let story = fixture.story(3_000_000).await;
    let idle = fixture.agent("hippolyte", "Hippolyte").await;

    let (status, empty) = fixture.get("/api/v1/budgets").await;
    assert_eq!(status, StatusCode::OK, "{empty}");
    assert_eq!(empty["spent_micros"], 3_000_000);
    assert!(empty["items"].as_array().unwrap().is_empty());

    let (status, saved) = fixture
        .put(
            "/api/v1/budgets",
            json!({
                "organization": {"monthly_limit_cents": 40000},
                "agents": [
                    {"agent_id": story.agent_id.0, "monthly_limit_cents": 250, "alert_threshold_percent": 90},
                    {"agent_id": idle.0, "monthly_limit_cents": 6000},
                ],
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let items = saved["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["scope"], "organization");
    assert_eq!(items[0]["spent_micros"], 3_000_000);
    assert_eq!(items[0]["percent"], 0);
    assert_eq!(items[0]["alert_threshold_percent"], 80);
    assert_eq!(items[1]["agent_name"], "Hippolyte");
    assert_eq!(items[1]["spent_micros"], 0);
    assert_eq!(items[2]["agent_name"], "Iris");
    assert_eq!(items[2]["spent_micros"], 3_000_000);
    assert_eq!(items[2]["percent"], 120);
    assert_eq!(items[2]["alert"], true);
    assert_eq!(items[2]["exceeded"], true);
    assert_eq!(items[2]["updated_by"], USER);

    let (status, replaced) = fixture
        .put(
            "/api/v1/budgets",
            json!({"organization": null, "agents": []}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(replaced["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn invalid_budgets_are_refused_and_change_nothing() {
    let Some(fixture) = fixture(Who::Admin).await else {
        return;
    };
    let Some(other) = fixture_agent_elsewhere().await else {
        return;
    };
    let (status, _) = fixture
        .put(
            "/api/v1/budgets",
            json!({"agents": [{"agent_id": other.0, "monthly_limit_cents": 100}]}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let mine = fixture.agent("iris", "Iris").await;
    let twice = json!({"agents": [
        {"agent_id": mine.0, "monthly_limit_cents": 100},
        {"agent_id": mine.0, "monthly_limit_cents": 200},
    ]});
    let (status, _) = fixture.put("/api/v1/budgets", twice).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .put(
            "/api/v1/budgets",
            json!({"organization": {"monthly_limit_cents": 0}}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .put(
            "/api/v1/budgets",
            json!({"organization": {"monthly_limit_cents": 100, "alert_threshold_percent": 101}}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, list) = fixture.get("/api/v1/budgets").await;
    assert!(list["items"].as_array().unwrap().is_empty());
}

async fn fixture_agent_elsewhere() -> Option<AgentId> {
    let other = fixture(Who::Admin).await?;
    Some(other.agent("iris", "Iris").await)
}

#[tokio::test]
async fn a_member_reads_budgets_but_only_an_admin_sets_them() {
    let Some(fixture) = fixture(Who::Member).await else {
        return;
    };
    let (status, _) = fixture.get("/api/v1/budgets").await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = fixture
        .put(
            "/api/v1/budgets",
            json!({"organization": {"monthly_limit_cents": 100}}),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

const COST_ROUTES: [&str; 4] = ["projects", "agents", "tasks", "users"];

#[tokio::test]
async fn costs_require_a_human_member() {
    let Some(fixture) = fixture(Who::Agent).await else {
        return;
    };
    for route in COST_ROUTES {
        let (status, _) = fixture
            .get(&format!("/api/v1/analytics/costs/{route}"))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{route}");
    }
}

#[tokio::test]
async fn costs_reject_an_empty_window() {
    let Some(fixture) = fixture(Who::Member).await else {
        return;
    };
    for route in COST_ROUTES {
        let (status, _) = fixture
            .get(&format!(
                "/api/v1/analytics/costs/{route}?from=2026-11-02T00:00:00Z&to=2026-11-01T00:00:00Z"
            ))
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{route}");
    }
}

#[tokio::test]
async fn costs_default_to_thirty_days() {
    let Some(fixture) = fixture(Who::Member).await else {
        return;
    };
    for route in COST_ROUTES {
        let (status, report) = fixture
            .get(&format!("/api/v1/analytics/costs/{route}"))
            .await;
        assert_eq!(status, StatusCode::OK, "{route}: {report}");
        let from: chrono::DateTime<Utc> = report["from"].as_str().unwrap().parse().unwrap();
        let to: chrono::DateTime<Utc> = report["to"].as_str().unwrap().parse().unwrap();
        assert_eq!(to - from, chrono::Duration::days(30), "{route}");
        assert_eq!(report["total"]["cost_micros"], 0);
        assert!(report["rows"].as_array().unwrap().is_empty());
    }
}

#[tokio::test]
async fn project_costs_return_agents_and_total() {
    let Some(fixture) = fixture(Who::Member).await else {
        return;
    };
    fixture.story(1_500).await;

    let (status, report) = fixture.get("/api/v1/analytics/costs/projects").await;
    assert_eq!(status, StatusCode::OK, "{report}");
    let rows = report["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["project_id"], Value::Null);
    assert_eq!(rows[0]["cost_micros"], 1_500);
    assert_eq!(rows[0]["agents"][0]["name"], "Iris");
    assert_eq!(rows[0]["agents"][0]["runs"], 1);
    let sum: i64 = rows
        .iter()
        .map(|r| r["cost_micros"].as_i64().unwrap())
        .sum();
    assert_eq!(report["total"]["cost_micros"], sum);
    assert!(report["total"]["input_tokens"].is_number());
    assert!(report["total"]["output_tokens"].is_number());

    let (_, agents) = fixture.get("/api/v1/analytics/costs/agents").await;
    assert_eq!(agents["rows"][0]["name"], "Iris");
    assert_eq!(agents["rows"][0]["cost_micros"], 1_500);
    let (_, tasks) = fixture.get("/api/v1/analytics/costs/tasks").await;
    assert_eq!(tasks["rows"][0]["title"], "Relancer les factures");
    assert_eq!(tasks["rows"][0]["agents"][0]["name"], "Iris");
}

#[tokio::test]
async fn tasks_with_a_foreign_project_are_not_found() {
    let Some(mine) = fixture(Who::Member).await else {
        return;
    };
    let Some(theirs) = fixture(Who::Member).await else {
        return;
    };
    let project = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO projects (id, organization_id, name, created_by, created_at, updated_at) VALUES ($1, $2, 'Autre', $3, now(), now())",
    )
    .bind(project)
    .bind(theirs.organization.id.0)
    .bind(USER)
    .execute(&theirs.pool)
    .await
    .unwrap();

    let (status, _) = mine
        .get(&format!(
            "/api/v1/analytics/costs/tasks?project_id={project}"
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
