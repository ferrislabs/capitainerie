#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use axum::Extension;
use capitainerie_auth::{
    AcceptedClients, Actor, ClientCredentials, FerrisKeyTokenExchange, FerrisKeyVerifier, Identity,
};
use capitainerie_common::{CardId, IntegrationInstanceId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::{
            Card, Column,
            service::{CreateCard, MoveCard},
        },
        client::{Client, service::CreateClient},
        integration::{
            ConnectorFailure, InstanceConnection, ProbeReport, action::ActionRequest,
            ports::IntegrationConnector, service::CreateInstance,
        },
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

struct Healthy;

impl IntegrationConnector for Healthy {
    async fn probe(
        &self,
        _connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        Ok(ProbeReport {
            server_version: "19.0".into(),
            warning: None,
        })
    }

    async fn perform(
        &self,
        _connection: &InstanceConnection,
        _request: &ActionRequest,
    ) -> Result<Value, ConnectorFailure> {
        Err(ConnectorFailure::NotSupported("not scripted".into()))
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
    fixture_as(None).await
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
    fn subject(&self, is_admin: bool) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: self.organization.id,
            actor: None,
            is_admin,
        }
    }

    async fn client(&self, name: &str) -> Client {
        self.usecase
            .create_client(
                self.subject(false),
                CreateClient {
                    name: name.into(),
                    aliases: vec![],
                    notes: String::new(),
                },
            )
            .await
            .unwrap()
    }

    async fn instance(
        &self,
        client: &Client,
        name: &str,
        environment: &str,
    ) -> IntegrationInstanceId {
        self.usecase
            .create_integration_instance(
                self.subject(true),
                CreateInstance {
                    client_id: client.id,
                    kind: "odoo_json2".into(),
                    name: name.into(),
                    environment: environment.into(),
                    base_url: "https://odoo.acme.test".into(),
                    database: None,
                    login: None,
                    secret: SecretToken::new("sk-0123456789abcdef-supersecret-wxyz"),
                },
                &Healthy,
            )
            .await
            .unwrap()
            .id
    }

    async fn card(&self, client: Option<&Client>, to: Column) -> Card {
        let proposed = self
            .usecase
            .create_card_linked(
                self.subject(false),
                CreateCard {
                    title: "Vérifier les factures d'avril".into(),
                    description: String::new(),
                    provenance: None,
                    run_id: None,
                    attributes: Default::default(),
                },
                client.map(|client| client.id),
                None,
            )
            .await
            .unwrap();
        if to == Column::Proposed {
            return proposed;
        }
        self.usecase
            .move_card(
                self.subject(false),
                proposed.id,
                MoveCard {
                    to,
                    expected_version: proposed.version,
                    run_id: None,
                },
            )
            .await
            .unwrap()
    }

    async fn intervene(&self, card_id: CardId, body: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!(
                "{}/api/v1/cards/{card_id}/interventions",
                self.base
            ))
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

fn request(card: &Card, grants: Value) -> Value {
    json!({"grants": grants, "expected_version": card.version})
}

struct Failure {
    status: u16,
    code: String,
    message: String,
}

async fn error_of(response: reqwest::Response) -> Failure {
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["status"], status);
    Failure {
        status,
        code: body["code"].as_str().unwrap().to_owned(),
        message: body["message"].as_str().unwrap().to_owned(),
    }
}

#[tokio::test]
async fn a_member_hands_a_todo_card_to_the_intervenant() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let acme = fixture.client("Acme").await;
    let staging = fixture.instance(&acme, "Odoo staging", "staging").await;
    let production = fixture.instance(&acme, "Odoo prod", "production").await;
    let card = fixture.card(Some(&acme), Column::Todo).await;

    let response = fixture
        .intervene(
            card.id,
            json!({
                "instructions": "  Ne modifie que le staging  ",
                "grants": [
                    {"instance_id": staging, "access": "write"},
                    {"instance_id": production, "access": "read"},
                ],
                "expected_version": card.version,
            }),
        )
        .await;

    assert_eq!(response.status(), 202);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["card"]["column"], "in_progress");
    assert_eq!(body["card"]["id"], card.id.0.to_string());
    assert_eq!(body["card"]["version"], card.version + 1);
    assert_eq!(body["card"]["client"]["id"], acme.id.0.to_string());
    let run_id: Uuid = body["run_id"].as_str().unwrap().parse().unwrap();
    let (status, pipeline, repository): (String, Vec<String>, Option<String>) =
        sqlx::query_as("SELECT status, pipeline, repository FROM runs WHERE id = $1")
            .bind(run_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!((status.as_str(), repository), ("queued", None));
    assert_eq!(pipeline, ["intervene"]);
    let (client_id, instructions): (Uuid, String) =
        sqlx::query_as("SELECT client_id, instructions FROM interventions WHERE run_id = $1")
            .bind(run_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(client_id, acme.id.0);
    assert_eq!(instructions, "Ne modifie que le staging");
    let mut grants: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT instance_id, access FROM intervention_grants WHERE run_id = $1")
            .bind(run_id)
            .fetch_all(&fixture.pool)
            .await
            .unwrap();
    grants.sort();
    let mut expected = vec![
        (staging.0, "write".to_owned()),
        (production.0, "read".to_owned()),
    ];
    expected.sort();
    assert_eq!(grants, expected);
}

#[tokio::test]
async fn an_instance_of_another_client_is_a_422_instance_of_another_client() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let acme = fixture.client("Acme").await;
    let globex = fixture.client("Globex").await;
    let odoo_prod = fixture.instance(&acme, "Odoo prod", "production").await;
    let card = fixture.card(Some(&globex), Column::Todo).await;

    let response = fixture
        .intervene(
            card.id,
            request(&card, json!([{"instance_id": odoo_prod, "access": "read"}])),
        )
        .await;

    let failure = error_of(response).await;
    assert_eq!(failure.status, 422);
    assert_eq!(failure.code, "instance_of_another_client");
    assert!(!failure.message.is_empty());
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn writing_to_production_without_the_confirmation_is_a_422_and_with_it_a_202() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let acme = fixture.client("Acme").await;
    let n8n_prod = fixture.instance(&acme, "n8n prod", "production").await;
    let card = fixture.card(Some(&acme), Column::Todo).await;
    let grants = json!([{"instance_id": n8n_prod, "access": "write"}]);

    let refused = fixture
        .intervene(card.id, request(&card, grants.clone()))
        .await;
    let failure = error_of(refused).await;
    assert_eq!(
        (failure.status, failure.code.as_str()),
        (422, "production_write_not_confirmed")
    );
    assert_eq!(fixture.runs(card.id).await, 0);

    let confirmed = fixture
        .intervene(
            card.id,
            json!({
                "grants": grants,
                "confirm_production_writes": true,
                "expected_version": card.version,
            }),
        )
        .await;
    assert_eq!(confirmed.status(), 202);
    assert_eq!(fixture.runs(card.id).await, 1);
}

#[tokio::test]
async fn every_other_refusal_is_a_422_with_its_code_and_creates_nothing() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let acme = fixture.client("Acme").await;
    let staging = fixture.instance(&acme, "Odoo staging", "staging").await;
    let archived = fixture.instance(&acme, "Odoo old", "staging").await;
    fixture
        .usecase
        .archive_integration_instance(fixture.subject(true), archived)
        .await
        .unwrap();
    let card = fixture.card(Some(&acme), Column::Todo).await;
    let unlinked = fixture.card(None, Column::Todo).await;
    let readable = json!([{"instance_id": staging, "access": "read"}]);
    let cases = [
        (&card, json!([]), "", "no_instance_granted"),
        (
            &card,
            json!([{"instance_id": archived, "access": "read"}]),
            "",
            "instance_archived",
        ),
        (
            &card,
            json!([{"instance_id": IntegrationInstanceId::new(), "access": "read"}]),
            "",
            "instance_of_another_client",
        ),
        (
            &card,
            readable.clone(),
            &"x".repeat(4001),
            "instructions_too_long",
        ),
        (&unlinked, readable, "", "card_without_client"),
    ];

    for (target, grants, instructions, code) in cases {
        let response = fixture
            .intervene(
                target.id,
                json!({
                    "instructions": instructions,
                    "grants": grants,
                    "expected_version": target.version,
                }),
            )
            .await;

        let failure = error_of(response).await;
        assert_eq!((failure.status, failure.code.as_str()), (422, code));
        assert_eq!(fixture.runs(target.id).await, 0, "{code}");
    }
}

#[tokio::test]
async fn too_many_grants_are_a_422() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let acme = fixture.client("Acme").await;
    let card = fixture.card(Some(&acme), Column::Todo).await;
    let grants: Vec<Value> = (0..101)
        .map(|_| json!({"instance_id": IntegrationInstanceId::new(), "access": "read"}))
        .collect();

    let response = fixture
        .intervene(card.id, request(&card, json!(grants)))
        .await;

    let failure = error_of(response).await;
    assert_eq!((failure.status, failure.code.as_str()), (422, "invalid"));
    assert_eq!(fixture.runs(card.id).await, 0);
}

#[tokio::test]
async fn the_conflicts_keep_their_codes() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let acme = fixture.client("Acme").await;
    let staging = fixture.instance(&acme, "Odoo staging", "staging").await;
    let grants = json!([{"instance_id": staging, "access": "read"}]);
    let proposed = fixture.card(Some(&acme), Column::Proposed).await;
    let card = fixture.card(Some(&acme), Column::Todo).await;

    let not_todo = fixture
        .intervene(proposed.id, request(&proposed, grants.clone()))
        .await;
    let stale = fixture
        .intervene(
            card.id,
            json!({"grants": grants, "expected_version": card.version - 1}),
        )
        .await;

    let failure = error_of(not_todo).await;
    assert_eq!(
        (failure.status, failure.message.as_str()),
        (409, "card_not_executable")
    );
    assert_eq!(error_of(stale).await.status, 409);
    assert_eq!(fixture.runs(card.id).await, 0);

    let first = fixture
        .intervene(card.id, request(&card, grants.clone()))
        .await;
    let moved: Value = first.json().await.unwrap();
    let version = moved["card"]["version"].as_i64().unwrap();
    fixture
        .usecase
        .move_card(
            fixture.subject(false),
            card.id,
            MoveCard {
                to: Column::Todo,
                expected_version: i32::try_from(version).unwrap(),
                run_id: None,
            },
        )
        .await
        .unwrap();
    let second = fixture
        .intervene(
            card.id,
            json!({"grants": grants, "expected_version": version + 1}),
        )
        .await;

    let failure = error_of(second).await;
    assert_eq!(
        (failure.status, failure.message.as_str()),
        (409, "execution_already_active")
    );
    assert_eq!(fixture.runs(card.id).await, 1);
}

#[tokio::test]
async fn an_unknown_card_is_a_404() {
    let Some(fixture) = fixture().await else {
        return;
    };

    let response = fixture
        .intervene(CardId::new(), json!({"grants": [], "expected_version": 1}))
        .await;

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn an_agent_is_a_403_and_nothing_is_created() {
    for actor in [
        "capitainerie-agent-integrator",
        "capitainerie-agent-captain",
        "capitainerie-agent-executor",
    ] {
        let Some(fixture) = fixture_as(Some(actor)).await else {
            return;
        };
        let acme = fixture.client("Acme").await;
        let staging = fixture.instance(&acme, "Odoo staging", "staging").await;
        let card = fixture.card(Some(&acme), Column::Todo).await;

        let response = fixture
            .intervene(
                card.id,
                request(&card, json!([{"instance_id": staging, "access": "read"}])),
            )
            .await;

        let failure = error_of(response).await;
        assert_eq!(
            (failure.status, failure.code.as_str()),
            (403, "forbidden"),
            "{actor}"
        );
        assert_eq!(fixture.runs(card.id).await, 0, "{actor}");
    }
}

#[tokio::test]
async fn a_card_of_another_organization_is_a_404() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let other = organization(&fixture.pool).await;
    let stranger = Subject {
        id: "user-9".into(),
        organization_id: other.id,
        actor: None,
        is_admin: false,
    };
    let foreign = fixture
        .usecase
        .create_card(
            stranger,
            CreateCard {
                title: "Ailleurs".into(),
                description: String::new(),
                provenance: None,
                run_id: None,
                attributes: Default::default(),
            },
        )
        .await
        .unwrap();

    let response = fixture
        .intervene(foreign.id, json!({"grants": [], "expected_version": 1}))
        .await;

    assert_eq!(response.status(), 404);
}
