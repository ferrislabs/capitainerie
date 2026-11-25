#![allow(clippy::unwrap_used)]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{Extension, Json, Router, routing::get, routing::post};
use capitainerie_auth::{AcceptedClients, FerrisKeyVerifier, Identity};
use capitainerie_common::{CardId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::{Card, edit::EditCard, service::CreateCard},
        client::links::Change,
        crew::{CrewAgent, service::CreateAgent},
        execution::{ExecutorStatus, GithubAppCredentials},
        provider::ProviderKind,
        user_provider::service::PutUserProvider,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_core::{
    domain::agent_identity::{AgentIdentity, ProvisionedClient, ports::AgentIdentityRepository},
    infrastructure::agent_identity::postgres::PgAgentIdentityRepository,
};
use capitainerie_github::{GithubApps, GithubBases};
use capitainerie_handlers::{AppState, Caller};
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

fn fresh_id() -> i64 {
    (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64
}

async fn spawn(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
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
            get(|| async {
                Json(
                    json!({"repositories": [{"full_name": "acme/web"}, {"full_name": "acme/api"}]}),
                )
            }),
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

struct Fixture {
    github_calls: Arc<AtomicUsize>,
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization: Organization,
    base: String,
}

async fn fixture() -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 3).await.unwrap();
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = format!("http://idp.test/realms/{id}");
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
        delegation: None,
    };
    let github_calls = Arc::new(AtomicUsize::new(0));
    let base = fake_github(Arc::clone(&github_calls)).await;
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
            actor: None,
            roles: vec![],
        },
        organization: organization.clone(),
        token: SecretToken::new("user-token"),
    };
    let app = capitainerie_handlers_cards::router(github)
        .layer(Extension(caller))
        .with_state(state);
    let fixture = Fixture {
        github_calls,
        pool,
        usecase,
        organization,
        base: spawn(app).await,
    };
    fixture.anthropic_key().await;
    Some(fixture)
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

    async fn anthropic_key(&self) {
        self.usecase
            .put_user_provider(PutUserProvider {
                subject: self.subject(false),
                kind: ProviderKind::Anthropic,
                secret: Some(SecretToken::new("sk-ant-api03-abcdefgh1234")),
                base_url: None,
                default_model: "claude-sonnet-5-5".into(),
                is_default: None,
            })
            .await
            .unwrap();
    }

    async fn connect_github(&self) {
        self.usecase
            .save_github_app(
                self.subject(true),
                GithubAppCredentials {
                    organization_id: self.organization.id,
                    app_id: fresh_id(),
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
        self.usecase
            .connect_github(self.subject(true), fresh_id(), "acme".into())
            .await
            .unwrap();
    }

    async fn agent(&self, slug: &str, repositories: &[&str]) -> CrewAgent {
        let agent = self
            .usecase
            .create_crew_agent(
                self.subject(false),
                CreateAgent {
                    slug: slug.into(),
                    name: slug.into(),
                    role: String::new(),
                    description: String::new(),
                    model: String::new(),
                    hue: 0,
                    reports_to: None,
                    instructions: "Tu audites.".into(),
                    connector_grants: vec![],
                    repository_grants: repositories.iter().map(|r| (*r).to_owned()).collect(),
                },
            )
            .await
            .unwrap();
        activate_identity(&self.pool, &SecretCipher::new(&[2u8; 32]).unwrap(), &agent).await;
        agent
    }

    async fn card(&self, agent: &CrewAgent) -> Card {
        let card = self
            .usecase
            .create_card_linked(
                self.subject(false),
                CreateCard {
                    title: "Audite le dépôt acme/web".into(),
                    description: "Selon ces instructions".into(),
                    provenance: None,
                    run_id: None,
                    attributes: Default::default(),
                },
                None,
                None,
            )
            .await
            .unwrap();
        let mut edit = EditCard::new(card.version);
        edit.assignee_agent = Change::Set(agent.id);
        self.usecase
            .edit_card(self.subject(false), card.id, edit)
            .await
            .unwrap()
    }

    async fn run(&self, card: &Card, extra: Value) -> reqwest::Response {
        let mut body = json!({"expected_version": card.version});
        for (key, value) in extra.as_object().unwrap() {
            body[key] = value.clone();
        }
        reqwest::Client::new()
            .post(format!("{}/api/v1/cards/{}/agent-runs", self.base, card.id))
            .json(&body)
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

    async fn executor(&self, enabled: bool) {
        self.usecase
            .record_executor_status(&ExecutorStatus {
                enabled,
                reason: (!enabled).then(|| "EXECUTOR_NAMESPACE n'est pas défini".to_owned()),
            })
            .await
            .unwrap();
    }
}

async fn failure(response: reqwest::Response) -> (u16, String, String) {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["status"], status);
    (
        status,
        body["code"].as_str().unwrap().to_owned(),
        body["message"].as_str().unwrap().to_owned(),
    )
}

#[tokio::test]
async fn a_repository_the_agent_was_not_granted_is_a_422_before_github_is_asked() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connect_github().await;
    let agent = fixture.agent("auditrice", &["acme/api"]).await;
    let card = fixture.card(&agent).await;

    let (status, code, message) =
        failure(fixture.run(&card, json!({"repository": "acme/web"})).await).await;

    assert_eq!((status, code.as_str()), (422, "repository_not_granted"));
    assert!(message.contains("acme/web"), "{message}");
    assert_eq!(fixture.runs(card.id).await, 0);
    assert_eq!(fixture.github_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn an_organization_without_github_is_a_409() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let agent = fixture.agent("auditrice", &["acme/web"]).await;
    let card = fixture.card(&agent).await;

    let (status, code, _) =
        failure(fixture.run(&card, json!({"repository": "acme/web"})).await).await;

    assert_eq!((status, code.as_str()), (409, "github_not_connected"));
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn a_malformed_repository_or_branch_is_a_422() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let agent = fixture.agent("auditrice", &["acme/web"]).await;
    let card = fixture.card(&agent).await;
    for extra in [
        json!({"repository": "acme"}),
        json!({"repository": "acme/web", "base_branch": "--upload-pack=evil"}),
    ] {
        let (status, _, _) = failure(fixture.run(&card, extra.clone()).await).await;
        assert_eq!(status, 422, "{extra}");
    }
    assert_eq!(fixture.runs(card.id).await, 0);
}

/// One test, because the executor status is the deployment's, not the
/// organization's: tests that flip it must not run side by side.
#[tokio::test]
async fn a_granted_repository_queues_an_execute_run_linked_to_the_agent() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connect_github().await;
    let agent = fixture.agent("auditrice", &["Acme/Web", "acme/api"]).await;

    fixture.executor(false).await;
    let card = fixture.card(&agent).await;
    let (status, code, message) =
        failure(fixture.run(&card, json!({"repository": "acme/web"})).await).await;
    fixture.executor(true).await;
    assert_eq!((status, code.as_str()), (409, "executor_disabled"));
    assert!(message.contains("EXECUTOR_NAMESPACE"), "{message}");
    assert_eq!(fixture.runs(card.id).await, 0);

    let response = fixture
        .run(
            &card,
            json!({"repository": "acme/web", "allow_writes": false}),
        )
        .await;
    assert_eq!(response.status(), 202);
    let run_id: Uuid = response.json::<Value>().await.unwrap()["run_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let (status, pipeline, repository, base_branch): (String, Vec<String>, String, String) =
        sqlx::query_as("SELECT status, pipeline, repository, base_branch FROM runs WHERE id = $1")
            .bind(run_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(status, "queued");
    assert_eq!(pipeline, ["execute"]);
    assert_eq!(repository, "acme/web");
    assert_eq!(base_branch, "trunk", "the default branch of the repository");
    let (agent_id, allow_writes): (Uuid, bool) =
        sqlx::query_as("SELECT agent_id, allow_writes FROM agent_tasks WHERE run_id = $1")
            .bind(run_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(agent_id, agent.id.0);
    assert!(!allow_writes, "audit mode");
    assert!(
        fixture
            .usecase
            .is_execution_run(capitainerie_common::RunId(run_id))
            .await
            .unwrap()
    );
    assert!(
        !fixture
            .usecase
            .is_agent_task_run(capitainerie_common::RunId(run_id))
            .await
            .unwrap()
    );

    let (status, code, message) =
        failure(fixture.run(&card, json!({"repository": "acme/web"})).await).await;
    assert_eq!(
        (status, code.as_str(), message.as_str()),
        (409, "conflict", "execution_already_active")
    );

    let other = fixture.card(&agent).await;
    let response = fixture
        .run(
            &other,
            json!({"repository": "acme/api", "base_branch": "release/2", "allow_writes": true}),
        )
        .await;
    assert_eq!(response.status(), 202);
    let (base_branch, allow_writes): (String, bool) = sqlx::query_as(
        "SELECT r.base_branch, t.allow_writes FROM runs r JOIN agent_tasks t ON t.run_id = r.id WHERE r.card_id = $1",
    )
    .bind(other.id.0)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(base_branch, "release/2");
    assert!(allow_writes, "change mode");
}

#[tokio::test]
async fn a_repository_the_installation_does_not_reach_is_a_403() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connect_github().await;
    let agent = fixture.agent("auditrice", &["acme/secret"]).await;
    let card = fixture.card(&agent).await;

    let response = fixture
        .run(&card, json!({"repository": "acme/secret"}))
        .await;

    // Either refused for the executor (when another test flipped it) or
    // refused by GitHub's reach; never queued.
    let (status, code, message) = failure(response).await;
    assert!(
        (status, message.as_str()) == (403, "repository_not_allowed")
            || (status, code.as_str()) == (409, "executor_disabled"),
        "{status} {code} {message}"
    );
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn an_agent_with_repositories_but_no_connector_still_needs_one_for_the_mcp_path() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let agent = fixture.agent("auditrice", &["acme/web"]).await;
    let card = fixture.card(&agent).await;

    let (status, code, _) = failure(fixture.run(&card, json!({})).await).await;

    assert_eq!((status, code.as_str()), (422, "agent_without_connector"));
}

/// Stands in for the worker's sweep: an agent runs only once FerrisKey holds
/// its client.
async fn activate_identity(pool: &PgPool, secrets: &SecretCipher, agent: &CrewAgent) {
    let now = chrono::Utc::now();
    let identity = AgentIdentity::pending(agent.organization_id, agent.id, now).activated(
        ProvisionedClient {
            client_uuid: "fk-test".into(),
            secret: SecretToken::new("agent-secret"),
        },
        now,
    );
    postgres::with_tx(pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, Some(secrets))
            .upsert(&identity)
            .await
    })
    .await
    .unwrap();
}
