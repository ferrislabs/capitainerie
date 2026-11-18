#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, Actor, FerrisKeyVerifier, Identity};
use capitainerie_common::{ApprovalId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        approval::{Approval, ApprovalKind, ApprovalStatus},
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
    let app: Router = capitainerie_handlers_approvals::router()
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

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::GET, path, None).await
    }
}

impl Fixture {
    async fn request(&self, title: &str) -> ApprovalId {
        let approval = self
            .usecase
            .request_approval(Approval {
                id: ApprovalId::new(),
                organization_id: self.organization.id,
                kind: ApprovalKind::AgentAction,
                title: title.into(),
                detail: json!({"reason": title}),
                requested_by_agent_id: None,
                card_id: None,
                run_id: None,
                status: ApprovalStatus::Pending,
                decided_by: None,
                decided_at: None,
                decision_note: None,
                created_at: Utc::now(),
            })
            .await
            .unwrap();
        approval.id
    }

    async fn announced(&self, id: ApprovalId, kind: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM outbox_events WHERE partition_key = $1 AND kind = $2",
        )
        .bind(id.0)
        .bind(kind)
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
async fn a_pending_approval_is_listed_decided_once_and_announced() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let first = fixture.request("Dépassement de budget").await;
    let second = fixture.request("Accès en écriture au Drive").await;
    assert_eq!(fixture.announced(first, "approval.requested").await, 1);

    let (status, pending) = fixture.get("/api/v1/approvals?status=pending").await;
    assert_eq!(status, StatusCode::OK, "{pending}");
    assert_eq!(
        titles(&pending),
        ["Accès en écriture au Drive", "Dépassement de budget"]
    );
    assert_eq!(pending["pending"], 2);
    let item = &pending["items"][1];
    assert_eq!(item["kind"], "agent_action");
    assert_eq!(item["status"], "pending");
    assert_eq!(item["detail"], json!({"reason": "Dépassement de budget"}));
    assert_eq!(item["decided_by"], Value::Null);

    let path = format!("/api/v1/approvals/{first}");
    let (status, decided) = fixture
        .post(
            &format!("{path}/decide"),
            json!({"decision": "reject", "note": " trop cher "}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_eq!(decided["status"], "rejected");
    assert_eq!(decided["decided_by"], "user-1");
    assert_eq!(decided["decision_note"], "trop cher");
    assert!(decided["decided_at"].is_string());
    assert_eq!(fixture.announced(first, "approval.decided").await, 1);

    let (status, _) = fixture
        .post(&format!("{path}/decide"), json!({"decision": "approve"}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(fixture.announced(first, "approval.decided").await, 1);
    let (_, detail) = fixture.get(&path).await;
    assert_eq!(detail["status"], "rejected");

    let (_, rejected) = fixture.get("/api/v1/approvals?status=rejected").await;
    assert_eq!(titles(&rejected), ["Dépassement de budget"]);
    assert_eq!(rejected["pending"], 1);
    let (_, all) = fixture.get("/api/v1/approvals").await;
    assert_eq!(all["items"].as_array().unwrap().len(), 2);

    let (status, approved) = fixture
        .post(
            &format!("/api/v1/approvals/{second}/decide"),
            json!({"decision": "approve"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(approved["status"], "approved");
    assert_eq!(approved["decision_note"], Value::Null);
    let (_, none) = fixture.get("/api/v1/approvals?status=pending").await;
    assert_eq!(none["pending"], 0);
    assert!(titles(&none).is_empty());
}

#[tokio::test]
async fn invalid_decisions_are_refused_and_change_nothing() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let id = fixture.request("Merge de la PR").await;
    let path = format!("/api/v1/approvals/{id}/decide");
    let (status, _) = fixture.post(&path, json!({"decision": "maybe"})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .post(
            &path,
            json!({"decision": "approve", "note": "a".repeat(2001)}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture.get("/api/v1/approvals?status=maybe").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, detail) = fixture.get(&format!("/api/v1/approvals/{id}")).await;
    assert_eq!(detail["status"], "pending");
    assert_eq!(fixture.announced(id, "approval.decided").await, 0);
}

#[tokio::test]
async fn an_approval_of_another_organization_is_a_404() {
    let Some(mine) = fixture(None).await else {
        return;
    };
    let Some(theirs) = fixture(None).await else {
        return;
    };
    let foreign = theirs.request("Envoi externe").await;
    let path = format!("/api/v1/approvals/{foreign}");
    let (status, _) = mine.get(&path).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = mine
        .post(&format!("{path}/decide"), json!({"decision": "approve"}))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = mine.get("/api/v1/approvals").await;
    assert!(titles(&list).is_empty());
    assert_eq!(list["pending"], 0);
    let (_, untouched) = theirs.get(&path).await;
    assert_eq!(untouched["status"], "pending");
}

#[tokio::test]
async fn an_agent_caller_cannot_read_or_decide() {
    let Some(fixture) = fixture(Some("agent:darnold")).await else {
        return;
    };
    let id = fixture.request("Merge de la PR").await;
    let (status, _) = fixture.get("/api/v1/approvals").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = fixture
        .post(
            &format!("/api/v1/approvals/{id}/decide"),
            json!({"decision": "approve"}),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let status: String = sqlx::query_scalar("SELECT status FROM approvals WHERE id = $1")
        .bind(id.0)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");
}
