#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use axum::Extension;
use capitainerie_auth::{
    AcceptedClients, Actor, ClientCredentials, FerrisKeyTokenExchange, FerrisKeyVerifier, Identity,
};
use capitainerie_common::{CardId, CoreError, McpConnectorId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::edit::EditCard,
        card::{Card, service::CreateCard},
        client::links::Change,
        crew::{
            CrewAgent,
            service::{CreateAgent, UpdateAgent},
        },
        mcp::{
            CodeExchange, Discovery, RefreshExchange, TokenSet,
            ports::McpOAuth,
            service::{CreateConnector, UpdateConnector},
        },
        provider::ProviderKind,
        user_provider::service::PutUserProvider,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApps, GithubBases};
use capitainerie_handlers::{AppState, Caller, Delegation};
use chrono::Utc;
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

const REDIRECT: &str = "https://console.example/connectors/oauth/callback";

struct FakeOAuth;

impl McpOAuth for FakeOAuth {
    async fn discover(&self, url: &str) -> Result<Discovery, CoreError> {
        Ok(Discovery {
            resource: url.to_owned(),
            issuer: "https://auth.example/realms/mestier".into(),
            authorization_endpoint: "https://auth.example/realms/mestier/auth".into(),
            token_endpoint: "https://auth.example/realms/mestier/token".into(),
            registration_endpoint: "https://auth.example/realms/mestier/register".into(),
            scopes: vec![],
        })
    }

    async fn register_client(&self, _: &str, _: &str, _: &[String]) -> Result<String, CoreError> {
        Ok("client-1".into())
    }

    async fn exchange_code(&self, _: &CodeExchange<'_>) -> Result<TokenSet, CoreError> {
        Ok(TokenSet {
            access_token: SecretToken::new("access"),
            refresh_token: None,
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            scope: None,
        })
    }

    async fn refresh(&self, _: &RefreshExchange<'_>) -> Result<TokenSet, CoreError> {
        Err(CoreError::Conflict("no refresh".into()))
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
    let issuer_url = format!("http://idp.test/realms/{id}");
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
    let fixture = fixture_as(None).await?;
    fixture.provider().await;
    Some(fixture)
}

async fn fixture_as(actor: Option<&str>) -> Option<Fixture> {
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
    let github = Arc::new(
        GithubApps::new(
            usecase.clone(),
            GithubBases {
                api_base: "http://127.0.0.1:1".into(),
                web_base: "http://127.0.0.1:1".into(),
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
    let app = capitainerie_handlers_cards::router(github)
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
    fn subject(&self) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: self.organization.id,
            actor: None,
            is_admin: false,
        }
    }

    async fn provider(&self) {
        self.usecase
            .put_user_provider(PutUserProvider {
                subject: self.subject(),
                kind: ProviderKind::Anthropic,
                secret: Some(SecretToken::new("sk-ant-api03-abcdefgh1234")),
                base_url: None,
                default_model: "claude-sonnet-5-5".into(),
                is_default: None,
            })
            .await
            .unwrap();
    }

    async fn connector(&self, slug: &str, connected: bool) -> McpConnectorId {
        let id = self
            .usecase
            .create_mcp_connector(
                self.subject(),
                CreateConnector {
                    url: format!("https://{slug}.example/mcp"),
                    name: Some(slug.to_owned()),
                    slug: Some(slug.to_owned()),
                },
                REDIRECT,
                &FakeOAuth,
            )
            .await
            .unwrap()
            .id;
        if connected {
            let start = self
                .usecase
                .start_mcp_authorization(self.subject(), id, REDIRECT)
                .await
                .unwrap();
            self.usecase
                .complete_mcp_authorization(self.subject(), &start.state, "code", &FakeOAuth)
                .await
                .unwrap();
        }
        id
    }

    async fn archive_connector(&self, id: McpConnectorId) {
        self.usecase
            .update_mcp_connector(
                self.subject(),
                id,
                UpdateConnector {
                    name: None,
                    archived: Some(true),
                },
            )
            .await
            .unwrap();
    }

    async fn agent(&self, slug: &str, grants: &[&str]) -> CrewAgent {
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
                    connector_grants: grants.iter().map(|grant| (*grant).to_owned()).collect(),
                },
            )
            .await
            .unwrap()
    }

    async fn card(&self, agent: Option<&CrewAgent>) -> Card {
        let card = self
            .usecase
            .create_card_linked(
                self.subject(),
                CreateCard {
                    title: "Relancer les factures".into(),
                    description: "Voir Mestier".into(),
                    provenance: None,
                    run_id: None,
                    attributes: Default::default(),
                },
                None,
                None,
            )
            .await
            .unwrap();
        let Some(agent) = agent else { return card };
        let mut edit = EditCard::new(card.version);
        edit.assignee_agent = Change::Set(agent.id);
        self.usecase
            .edit_card(self.subject(), card.id, edit)
            .await
            .unwrap()
    }

    async fn run(&self, card_id: CardId, body: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/api/v1/cards/{card_id}/agent-runs", self.base))
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
}

fn body(card: &Card) -> Value {
    json!({"expected_version": card.version, "allow_writes": false})
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
async fn a_member_runs_the_assigned_agent_of_a_card() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connector("mestier", true).await;
    let agent = fixture.agent("darnold", &["mestier"]).await;
    let card = fixture.card(Some(&agent)).await;

    let response = fixture
        .run(
            card.id,
            json!({"expected_version": card.version, "allow_writes": true}),
        )
        .await;

    assert_eq!(response.status(), 202);
    let created: Value = response.json().await.unwrap();
    let run_id: Uuid = created["run_id"].as_str().unwrap().parse().unwrap();
    let (status, pipeline, initiated_by, repository): (
        String,
        Vec<String>,
        String,
        Option<String>,
    ) = sqlx::query_as("SELECT status, pipeline, initiated_by, repository FROM runs WHERE id = $1")
        .bind(run_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(status, "queued");
    assert_eq!(pipeline, ["agent_task"]);
    assert_eq!(initiated_by, "user-1");
    assert_eq!(repository, None);
    let (agent_id, allow_writes): (Uuid, bool) =
        sqlx::query_as("SELECT agent_id, allow_writes FROM agent_tasks WHERE run_id = $1")
            .bind(run_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(agent_id, agent.id.0);
    assert!(allow_writes);
}

#[tokio::test]
async fn allow_writes_defaults_to_false() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connector("mestier", true).await;
    let agent = fixture.agent("darnold", &["mestier"]).await;
    let card = fixture.card(Some(&agent)).await;

    let response = fixture
        .run(card.id, json!({"expected_version": card.version}))
        .await;

    assert_eq!(response.status(), 202);
    let allowed: bool =
        sqlx::query_scalar("SELECT allow_writes FROM agent_tasks WHERE card_id = $1")
            .bind(card.id.0)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert!(!allowed);
}

#[tokio::test]
async fn a_stale_version_is_a_409_and_queues_nothing() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connector("mestier", true).await;
    let agent = fixture.agent("darnold", &["mestier"]).await;
    let card = fixture.card(Some(&agent)).await;

    let response = fixture
        .run(card.id, json!({"expected_version": card.version - 1}))
        .await;

    let (status, code, _) = failure(response).await;
    assert_eq!((status, code.as_str()), (409, "conflict"));
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn a_card_without_an_agent_is_a_422() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let card = fixture.card(None).await;

    let (status, code, _) = failure(fixture.run(card.id, body(&card)).await).await;

    assert_eq!((status, code.as_str()), (422, "card_without_agent"));
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn an_archived_agent_is_a_422() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connector("mestier", true).await;
    let agent = fixture.agent("darnold", &["mestier"]).await;
    let card = fixture.card(Some(&agent)).await;
    fixture
        .usecase
        .update_crew_agent(
            fixture.subject(),
            agent.id,
            UpdateAgent {
                archived: Some(true),
                ..UpdateAgent::default()
            },
        )
        .await
        .unwrap();

    let (status, code, _) = failure(fixture.run(card.id, body(&card)).await).await;

    assert_eq!((status, code.as_str()), (422, "agent_archived"));
}

#[tokio::test]
async fn an_agent_without_an_active_connector_is_a_422() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let ungranted = fixture.agent("sans-outil", &[]).await;
    let card = fixture.card(Some(&ungranted)).await;
    let (status, code, _) = failure(fixture.run(card.id, body(&card)).await).await;
    assert_eq!((status, code.as_str()), (422, "agent_without_connector"));

    let archived = fixture.connector("ancien", true).await;
    fixture.archive_connector(archived).await;
    let agent = fixture.agent("darnold", &["ancien", "inconnu"]).await;
    let card = fixture.card(Some(&agent)).await;
    let (status, code, _) = failure(fixture.run(card.id, body(&card)).await).await;
    assert_eq!((status, code.as_str()), (422, "agent_without_connector"));
}

#[tokio::test]
async fn a_missing_connection_is_a_409_naming_the_connector() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connector("mestier", true).await;
    fixture.connector("odoo-demo", false).await;
    let agent = fixture.agent("darnold", &["mestier", "odoo-demo"]).await;
    let card = fixture.card(Some(&agent)).await;

    let (status, code, message) = failure(fixture.run(card.id, body(&card)).await).await;

    assert_eq!((status, code.as_str()), (409, "not_connected"));
    assert!(message.contains("odoo-demo"), "{message}");
    assert!(!message.contains("mestier"), "{message}");
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn a_second_active_run_on_the_card_is_a_409() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.connector("mestier", true).await;
    let agent = fixture.agent("darnold", &["mestier"]).await;
    let card = fixture.card(Some(&agent)).await;
    assert_eq!(fixture.run(card.id, body(&card)).await.status(), 202);

    let (status, code, message) = failure(fixture.run(card.id, body(&card)).await).await;

    assert_eq!((status, code.as_str()), (409, "conflict"));
    assert_eq!(message, "execution_already_active");
    assert_eq!(fixture.runs(card.id).await, 1);
}

#[tokio::test]
async fn an_agent_caller_is_refused() {
    let Some(fixture) = fixture_as(Some("capitainerie-agent-writer")).await else {
        return;
    };
    fixture.connector("mestier", true).await;
    let agent = fixture.agent("darnold", &["mestier"]).await;
    let card = fixture.card(Some(&agent)).await;

    let (status, code, _) = failure(fixture.run(card.id, body(&card)).await).await;

    assert_eq!((status, code.as_str()), (403, "forbidden"));
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn a_card_of_another_organization_is_a_404() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let other = organization(&fixture.pool).await;
    let foreign = fixture
        .usecase
        .create_card_linked(
            Subject {
                id: "user-9".into(),
                organization_id: other.id,
                actor: None,
                is_admin: false,
            },
            CreateCard {
                title: "Ailleurs".into(),
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

    let (status, code, _) = failure(fixture.run(foreign.id, body(&foreign)).await).await;

    assert_eq!((status, code.as_str()), (404, "not_found"));
    assert_eq!(fixture.runs(foreign.id).await, 0);
}

#[tokio::test]
async fn an_unknown_card_is_a_404_and_a_malformed_body_is_refused() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let (status, code, _) = failure(
        fixture
            .run(CardId::new(), json!({"expected_version": 1}))
            .await,
    )
    .await;
    assert_eq!((status, code.as_str()), (404, "not_found"));
    let malformed = fixture
        .run(CardId::new(), json!({"allow_writes": true}))
        .await;
    assert!(malformed.status().is_client_error());
}

#[tokio::test]
async fn without_a_provider_of_the_callers_or_the_organizations_the_run_is_a_409_no_provider() {
    let Some(fixture) = fixture_as(None).await else {
        return;
    };
    fixture.connector("mestier", true).await;
    let agent = fixture.agent("darnold", &["mestier"]).await;
    let card = fixture.card(Some(&agent)).await;

    let refused = fixture.run(card.id, body(&card)).await;
    let (status, code, message) = failure(refused).await;

    assert_eq!(status, 409);
    assert_eq!(code, "no_provider");
    assert_eq!(
        message,
        "Aucun fournisseur LLM connecté : ajoutez votre clé dans Paramètres > Fournisseurs"
    );
    assert_eq!(fixture.runs(card.id).await, 0);

    fixture.provider().await;
    let accepted = fixture.run(card.id, body(&card)).await;
    assert_eq!(accepted.status(), 202);
    assert_eq!(fixture.runs(card.id).await, 1);
}
