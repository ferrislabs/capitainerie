#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, FerrisKeyVerifier, Identity};
use capitainerie_common::{OrganizationId, RunId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{agent::SecretToken, authorization::Subject, run::service::SubmitTranscript},
    infrastructure::postgres,
};
use capitainerie_handlers::{AppState, Caller};
use chrono::{Duration, Utc};
use serde_json::Value;
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

async fn fixture() -> Option<Fixture> {
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
            actor: None,
            roles: vec![],
        },
        organization: organization.clone(),
        token: SecretToken::new("user-token"),
    };
    let app: Router = capitainerie_handlers_runs::router()
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

impl Fixture {
    async fn submit(&self, organization_id: OrganizationId, title: &str) -> RunId {
        self.usecase
            .submit_transcript(SubmitTranscript {
                subject: Subject {
                    id: "user-1".into(),
                    organization_id,
                    actor: None,
                    is_admin: false,
                },
                title: title.into(),
                body: "Alice : je m'occupe du devis.".into(),
            })
            .await
            .unwrap()
            .run_id
    }

    async fn age(&self, run: RunId, minutes: i64) {
        sqlx::query("UPDATE runs SET created_at = $2 WHERE id = $1")
            .bind(run.0)
            .bind(Utc::now() - Duration::minutes(minutes))
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn set_status(&self, run: RunId, status: &str) {
        sqlx::query("UPDATE runs SET status = $2 WHERE id = $1")
            .bind(run.0)
            .bind(status)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn step(
        &self,
        run: RunId,
        organization_id: OrganizationId,
        seq: i32,
        kind: &str,
        status: &str,
        tokens: (i64, i64, i64),
    ) {
        sqlx::query(
            "INSERT INTO run_steps (id, run_id, organization_id, seq, kind, agent, status, attempt, model, input_tokens, output_tokens, cost_micros, started_at)
             VALUES ($1, $2, $3, $4, $5, 'extractor', $6, 1, 'model-x', $7, $8, $9, now())",
        )
        .bind(Uuid::now_v7())
        .bind(run.0)
        .bind(organization_id.0)
        .bind(seq)
        .bind(kind)
        .bind(status)
        .bind(tokens.0)
        .bind(tokens.1)
        .bind(tokens.2)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    async fn active(&self) -> (reqwest::StatusCode, Value) {
        let response = reqwest::get(format!("{}/api/v1/runs/active", self.base))
            .await
            .unwrap();
        let status = response.status();
        (status, response.json().await.unwrap())
    }
}

#[tokio::test]
async fn only_the_callers_active_runs_come_back_oldest_first() {
    let Some(f) = fixture().await else { return };
    let other = organization(&f.pool).await;
    let oldest = f.submit(f.organization.id, "Le plus ancien").await;
    let newest = f.submit(f.organization.id, "Le plus récent").await;
    let finished = f.submit(f.organization.id, "Terminé").await;
    let foreign = f.submit(other.id, "Autre organisation").await;
    f.age(oldest, 30).await;
    f.age(newest, 10).await;
    f.set_status(newest, "running").await;
    f.set_status(finished, "succeeded").await;
    f.set_status(foreign, "running").await;

    let (status, body) = f.active().await;

    assert_eq!(status, 200);
    let ids: Vec<&str> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["run"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [oldest.0.to_string(), newest.0.to_string()]);
    assert_eq!(body["items"][0]["run"]["status"], "queued");
    assert_eq!(body["items"][1]["run"]["status"], "running");
}

#[tokio::test]
async fn a_queued_run_has_no_current_step() {
    let Some(f) = fixture().await else { return };
    f.submit(f.organization.id, "En file").await;

    let (_, body) = f.active().await;

    let item = &body["items"][0];
    assert_eq!(item["current_step"], Value::Null);
    assert_eq!(item["steps_done"], 0);
    assert_eq!(item["steps_total"], 3);
    assert_eq!(item["input_tokens"], 0);
    assert_eq!(item["cost_micros"], 0);
}

#[tokio::test]
async fn a_running_run_reports_its_current_step_and_sums_every_step() {
    let Some(f) = fixture().await else { return };
    let run = f.submit(f.organization.id, "En cours").await;
    f.set_status(run, "running").await;
    let org = f.organization.id;
    f.step(run, org, 0, "extract", "succeeded", (1_000, 200, 50))
        .await;
    f.step(run, org, 1, "deduplicate", "skipped", (0, 0, 0))
        .await;
    f.step(run, org, 2, "write", "running", (30, 4, 6)).await;

    let (_, body) = f.active().await;

    let item = &body["items"][0];
    assert_eq!(item["current_step"]["kind"], "write");
    assert_eq!(item["current_step"]["agent"], "extractor");
    assert_eq!(item["current_step"]["model"], "model-x");
    assert!(item["current_step"]["started_at"].is_string());
    assert_eq!(item["steps_done"], 2);
    assert_eq!(item["steps_total"], 3);
    assert_eq!(item["input_tokens"], 1_030);
    assert_eq!(item["output_tokens"], 204);
    assert_eq!(item["cost_micros"], 56);
}

#[tokio::test]
async fn the_active_route_does_not_shadow_a_run_id() {
    let Some(f) = fixture().await else { return };
    let run = f.submit(f.organization.id, "Un run").await;

    let response = reqwest::get(format!("{}/api/v1/runs/{}", f.base, run.0))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["id"], run.0.to_string());
}
