#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use axum::{Extension, Json, Router, routing::get, routing::post};
use capitainerie_auth::{
    AcceptedClients, ClientCredentials, FerrisKeyTokenExchange, FerrisKeyVerifier, Identity,
};
use capitainerie_common::{CardId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::{
            Card, Column,
            service::{CreateCard, MoveCard},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApp, GithubClient};
use capitainerie_handlers::{AppState, Caller, Delegation};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const PEM: &str = include_str!("../../github/tests/fixtures/test-only-app-key.pem");

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

async fn spawn(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
}

async fn fake_identity_provider() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let endpoint = format!("{base}/token");
    let app = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(move || async move { Json(json!({"token_endpoint": endpoint})) }),
        )
        .route(
            "/token",
            post(|| async { Json(json!({"access_token": "executor-token", "expires_in": 1800})) }),
        );
    tokio::spawn(async move { axum::serve(listener, app).await });
    base
}

async fn fake_github() -> String {
    let app = Router::new()
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
            get(|| async { Json(json!({"repositories": [{"full_name": "acme/web"}]})) }),
        )
        .route(
            "/repos/{owner}/{repo}",
            get(|| async { Json(json!({"default_branch": "trunk"})) }),
        );
    spawn(app).await
}

struct Fixture {
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization: Organization,
    base: String,
}

async fn fixture(with_github_app: bool) -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 3).await.unwrap();
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = fake_identity_provider().await;
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
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap());
    let state = AppState {
        usecase: usecase.clone(),
        verifier: FerrisKeyVerifier::new(AcceptedClients::new(["capitainerie-console"])),
        delegation: Some(Delegation {
            exchanger: FerrisKeyTokenExchange::new(
                Some(ClientCredentials {
                    client_id: "capitainerie-api".into(),
                    client_secret: SecretToken::new("secret"),
                }),
                vec![],
                "capitainerie-mcp",
            ),
            agents: vec![],
        }),
    };
    let github = if with_github_app {
        let mut app = GithubApp::new(1, SecretToken::new(PEM));
        app.api_base = fake_github().await;
        Some(Arc::new(GithubClient::new(app).unwrap()))
    } else {
        None
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
    let app = capitainerie_handlers_cards::router(github)
        .layer(Extension(caller))
        .with_state(state);
    Some(Fixture {
        pool,
        usecase,
        organization,
        base: spawn(app).await,
    })
}

impl Fixture {
    fn subject(&self, is_admin: bool) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: self.organization.id,
            actor: None,
            is_admin,
        }
    }

    async fn connect(&self) {
        self.usecase
            .connect_github(self.subject(true), 42, "acme".into())
            .await
            .unwrap();
    }

    async fn proposed_card(&self) -> Card {
        self.usecase
            .create_card(
                self.subject(false),
                CreateCard {
                    title: "Ajouter un CHANGELOG".into(),
                    description: "À la racine".into(),
                    provenance: None,
                    run_id: None,
                },
            )
            .await
            .unwrap()
    }

    async fn todo_card(&self) -> Card {
        let proposed = self.proposed_card().await;
        self.usecase
            .move_card(
                self.subject(false),
                proposed.id,
                MoveCard {
                    to: Column::Todo,
                    expected_version: proposed.version,
                    run_id: None,
                },
            )
            .await
            .unwrap()
    }

    async fn execute(&self, card: &Card, repository: &str, version: i32) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!(
                "{}/api/v1/cards/{}/executions",
                self.base, card.id.0
            ))
            .json(&json!({"repository": repository, "expected_version": version}))
            .send()
            .await
            .unwrap()
    }

    async fn runs(&self, card_id: CardId) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM runs WHERE card_id = $1")
            .bind(card_id.0)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

async fn message(response: reqwest::Response) -> String {
    let body: Value = response.json().await.unwrap();
    body["message"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_member_hands_a_todo_card_to_the_executor() {
    let Some(fixture) = fixture(true).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;

    let response = fixture.execute(&card, "acme/web", card.version).await;

    assert_eq!(response.status(), 202);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["card"]["column"], "in_progress");
    assert_eq!(body["card"]["id"], card.id.0.to_string());
    let run_id: Uuid = body["run_id"].as_str().unwrap().parse().unwrap();
    let (status, repository, base_branch): (String, String, String) =
        sqlx::query_as("SELECT status, repository, base_branch FROM runs WHERE id = $1")
            .bind(run_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(
        (status.as_str(), repository.as_str(), base_branch.as_str()),
        ("queued", "acme/web", "trunk")
    );
}

#[tokio::test]
async fn a_proposed_card_cannot_be_handed_over_and_nothing_is_created() {
    let Some(fixture) = fixture(true).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.proposed_card().await;

    let response = fixture.execute(&card, "acme/web", card.version).await;

    assert_eq!(response.status(), 409);
    assert_eq!(message(response).await, "card_not_executable");
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn a_second_execution_of_the_same_card_is_refused() {
    let Some(fixture) = fixture(true).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;
    let first = fixture.execute(&card, "acme/web", card.version).await;
    let moved: Value = first.json().await.unwrap();

    let version = i32::try_from(moved["card"]["version"].as_i64().unwrap()).unwrap();
    let second = fixture.execute(&card, "acme/web", version).await;

    assert_eq!(second.status(), 409);
    assert_eq!(message(second).await, "execution_already_active");
    assert_eq!(fixture.runs(card.id).await, 1);
}

#[tokio::test]
async fn without_a_connected_installation_the_answer_is_github_not_connected() {
    let Some(fixture) = fixture(true).await else {
        return;
    };
    let card = fixture.todo_card().await;

    let response = fixture.execute(&card, "acme/web", card.version).await;

    assert_eq!(response.status(), 409);
    assert_eq!(message(response).await, "github_not_connected");
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn without_a_github_app_nothing_is_connected_either() {
    let Some(fixture) = fixture(false).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;

    let response = fixture.execute(&card, "acme/web", card.version).await;

    assert_eq!(response.status(), 409);
    assert_eq!(message(response).await, "github_not_connected");
}

#[tokio::test]
async fn a_repository_outside_the_installation_is_refused() {
    let Some(fixture) = fixture(true).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;

    let outside = fixture.execute(&card, "acme/secret", card.version).await;
    let malformed = fixture
        .execute(&card, "not-a-repository", card.version)
        .await;

    assert_eq!(outside.status(), 403);
    assert_eq!(message(outside).await, "repository_not_allowed");
    assert_eq!(malformed.status(), 422);
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn a_stale_version_is_a_plain_conflict() {
    let Some(fixture) = fixture(true).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;

    let response = fixture.execute(&card, "acme/web", card.version + 7).await;

    assert_eq!(response.status(), 409);
    assert_eq!(fixture.runs(card.id).await, 0);
}
