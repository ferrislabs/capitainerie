#![allow(clippy::unwrap_used)]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

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
        execution::GithubAppCredentials,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApps, GithubBases};
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

fn fresh_installation() -> i64 {
    (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64
}

async fn spawn(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
}

/// Served under a per-organization path: a port the OS hands out again must
/// not repeat an issuer URL, which is unique in `organizations`.
async fn fake_identity_provider(realm: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}/realms/{realm}", listener.local_addr().unwrap());
    let endpoint = format!("{issuer}/token");
    let routes = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(move || async move { Json(json!({"token_endpoint": endpoint})) }),
        )
        .route(
            "/token",
            post(|| async { Json(json!({"access_token": "executor-token", "expires_in": 1800})) }),
        );
    let app = Router::new().nest(&format!("/realms/{realm}"), routes);
    tokio::spawn(async move { axum::serve(listener, app).await });
    issuer
}

async fn fake_github(calls: Arc<AtomicUsize>) -> String {
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
        )
        .layer(axum::middleware::from_fn(
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    next.run(request).await
                }
            },
        ));
    spawn(app).await
}

#[derive(Clone, Copy, PartialEq)]
enum Github {
    Absent,
    Fake,
    Unreachable,
}

struct Fixture {
    github_calls: Arc<AtomicUsize>,
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization: Organization,
    base: String,
}

async fn fixture(with_github_app: bool) -> Option<Fixture> {
    let github = if with_github_app {
        Github::Fake
    } else {
        Github::Absent
    };
    fixture_as(github, None).await
}

async fn fixture_as(github: Github, actor: Option<&str>) -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 3).await.unwrap();
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = fake_identity_provider(&slug).await;
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
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap())
        .with_secrets_cipher(SecretCipher::new(&[2u8; 32]).unwrap());
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
    let github_calls = Arc::new(AtomicUsize::new(0));
    let base = match github {
        Github::Absent => "http://127.0.0.1:1".to_owned(),
        Github::Fake => fake_github(Arc::clone(&github_calls)).await,
        Github::Unreachable => "http://127.0.0.1:1".to_owned(),
    };
    if github != Github::Absent {
        usecase
            .save_github_app(
                Subject {
                    id: "user-1".into(),
                    organization_id: organization.id,
                    actor: None,
                    is_admin: true,
                },
                GithubAppCredentials {
                    organization_id: organization.id,
                    app_id: fresh_installation(),
                    slug: "capitainerie-test".into(),
                    client_id: "Iv1.test".into(),
                    client_secret: SecretToken::new("oauth-secret"),
                    private_key_pem: SecretToken::new(PEM),
                    webhook_secret: SecretToken::new("hook"),
                    html_url: "https://github.com/apps/capitainerie-test".into(),
                    owner_login: "acme".into(),
                    owner_type: "Organization".into(),
                    created_by: "user-1".into(),
                    created_at: Utc::now(),
                },
            )
            .await
            .unwrap();
    }
    let github = Arc::new(
        GithubApps::new(
            usecase.clone(),
            GithubBases {
                api_base: base.clone(),
                web_base: base,
            },
        )
        .unwrap(),
    );
    let caller = Caller {
        identity: Identity {
            subject: "user-1".into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: actor.map(|client_id| capitainerie_auth::Actor {
                sub: "service-account".into(),
                client_id: Some(client_id.to_owned()),
                act: None,
            }),
            roles: vec![],
        },
        organization: organization.clone(),
        token: SecretToken::new("user-token"),
    };
    let app = capitainerie_handlers_cards::router(github)
        .layer(Extension(caller))
        .with_state(state);
    Some(Fixture {
        github_calls,
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
            .connect_github(self.subject(true), fresh_installation(), "acme".into())
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

#[tokio::test]
async fn a_proposed_card_is_refused_before_github_is_asked_anything() {
    let Some(fixture) = fixture_as(Github::Unreachable, None).await else {
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
async fn an_agent_is_refused_before_github_is_asked_anything() {
    let Some(fixture) = fixture_as(Github::Fake, Some("capitainerie-agent-executor")).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;

    let response = fixture.execute(&card, "acme/web", card.version).await;

    assert_eq!(response.status(), 403);
    assert_eq!(fixture.github_calls.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn an_unknown_card_is_a_404_before_github_is_asked_anything() {
    let Some(fixture) = fixture_as(Github::Fake, None).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;
    let unknown = Card {
        id: CardId::new(),
        ..card
    };

    let response = fixture.execute(&unknown, "acme/web", 1).await;

    assert_eq!(response.status(), 404);
    assert_eq!(fixture.github_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn an_executable_card_reaches_github_after_the_checks() {
    let Some(fixture) = fixture_as(Github::Fake, None).await else {
        return;
    };
    fixture.connect().await;
    let card = fixture.todo_card().await;

    let response = fixture.execute(&card, "acme/web", card.version).await;

    assert_eq!(response.status(), 202);
    assert!(fixture.github_calls.load(Ordering::SeqCst) > 0);
}
