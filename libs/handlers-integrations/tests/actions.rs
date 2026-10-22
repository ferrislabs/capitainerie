#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use axum::{Extension, Router};
use capitainerie_auth::{Actor, Identity};
use capitainerie_common::{CardId, ClientId, OrganizationId, RunId};
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
            ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport,
            action::{Access, ActionCall, ActionRequest},
            intervention::ConfideCard,
            journal::{ActionContext, Grant, INTEGRATOR_AGENT},
            ports::IntegrationConnector,
            service::CreateInstance,
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApps, GithubBases};
use capitainerie_handlers::Caller;
use capitainerie_handlers_integrations::GithubIntegration;
use capitainerie_integrations::{HttpConnector, NetworkPolicy};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const KEY: &str = "sk-0123456789abcdef-supersecret-wxyz";

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
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

fn caller(organization: &Organization, actor: Option<&str>) -> Caller {
    Caller {
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
        token: SecretToken::new("token"),
    }
}

fn subject(organization: &Organization, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id: organization.id,
        actor: None,
        is_admin,
    }
}

fn usecase_of(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[5u8; 32]).unwrap())
}

struct Scripted;

impl IntegrationConnector for Scripted {
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
        request: &ActionRequest,
    ) -> Result<Value, ConnectorFailure> {
        match request.access() {
            Access::Read => Ok(json!([{"id": 7, "name": "Acme", "api_key": "sk-never-shown"}])),
            Access::Write => Ok(json!(true)),
        }
    }
}

async fn serve(pool: &PgPool, caller: Caller) -> String {
    let usecase = usecase_of(pool);
    let apps = GithubApps::new(
        usecase.clone(),
        GithubBases {
            api_base: "http://127.0.0.1:9".into(),
            web_base: "http://127.0.0.1:9".into(),
        },
    )
    .unwrap();
    let github = GithubIntegration::new(Arc::new(apps));
    let connector = HttpConnector::new(NetworkPolicy::default()).unwrap();
    let app: Router = capitainerie_handlers_integrations::router::<()>(usecase, github, connector)
        .layer(Extension(caller));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
}

struct World {
    pool: PgPool,
    organization: Organization,
    usecase: CapitainerieUseCase,
    client: Client,
    card: Card,
    run_id: RunId,
    staging: IntegrationInstance,
    production: IntegrationInstance,
}

async fn instance(
    world: (&CapitainerieUseCase, &Organization, &Client),
    name: &str,
    environment: &str,
) -> IntegrationInstance {
    let (usecase, organization, client) = world;
    usecase
        .create_integration_instance(
            subject(organization, true),
            CreateInstance {
                client_id: client.id,
                kind: "odoo_json2".into(),
                name: name.into(),
                environment: environment.into(),
                base_url: "https://odoo.acme.test".into(),
                database: None,
                login: None,
                secret: SecretToken::new(KEY),
            },
            &Scripted,
        )
        .await
        .unwrap()
}

async fn world() -> Option<World> {
    let url = database_url().or_else(|| {
        eprintln!("skipped: DATABASE_URL is unset");
        None
    })?;
    let pool = postgres::connect(&url, 3).await.unwrap();
    let organization = organization(&pool).await;
    let usecase = usecase_of(&pool);
    let client = usecase
        .create_client(
            subject(&organization, false),
            CreateClient {
                name: "Acme".into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap();
    let staging = instance(
        (&usecase, &organization, &client),
        "Odoo staging",
        "staging",
    )
    .await;
    let production = instance(
        (&usecase, &organization, &client),
        "Odoo prod",
        "production",
    )
    .await;
    let proposed = usecase
        .create_card_linked(
            subject(&organization, false),
            CreateCard {
                title: "Vérifier les partenaires".into(),
                description: String::new(),
                provenance: None,
                run_id: None,
                attributes: Default::default(),
            },
            Some(client.id),
            None,
        )
        .await
        .unwrap();
    let card = usecase
        .move_card(
            subject(&organization, false),
            proposed.id,
            MoveCard {
                to: Column::Todo,
                expected_version: proposed.version,
                run_id: None,
            },
        )
        .await
        .unwrap();
    let started = usecase
        .confide_card_to_integrator(
            subject(&organization, false),
            card.id,
            ConfideCard {
                instructions: String::new(),
                grants: vec![(staging.id, Access::Write), (production.id, Access::Read)],
                confirm_production_writes: false,
                expected_version: card.version,
            },
        )
        .await
        .unwrap();
    Some(World {
        pool,
        organization,
        usecase,
        client,
        card,
        run_id: started.run_id,
        staging,
        production,
    })
}

impl World {
    fn context(&self) -> ActionContext {
        ActionContext {
            subject: Subject {
                actor: Some(INTEGRATOR_AGENT.into()),
                ..subject(&self.organization, false)
            },
            run_id: Some(self.run_id),
            card_id: Some(self.card.id),
            client_id: self.client.id,
            grants: vec![
                Grant {
                    instance_id: self.staging.id,
                    access: Access::Write,
                },
                Grant {
                    instance_id: self.production.id,
                    access: Access::Read,
                },
            ],
            reverts_action_id: None,
        }
    }

    async fn call(&self, tool: &str, arguments: Value) {
        let call = ActionCall::parse(tool, &arguments).unwrap();
        self.usecase
            .perform_integration_action(&self.context(), call, &Scripted)
            .await
            .unwrap();
    }

    async fn journal(&self) {
        self.call(
            "odoo_count",
            json!({"instance": self.production.id, "model": "res.partner", "domain": []}),
        )
        .await;
        self.call(
            "odoo_write",
            json!({"instance": self.production.id, "model": "res.partner", "ids": [7], "values": {"name": "Acme SA"}}),
        )
        .await;
        self.call(
            "odoo_write",
            json!({"instance": self.staging.id, "model": "res.partner", "ids": [7], "values": {"name": "Acme SA"}}),
        )
        .await;
    }

    async fn listing(&self, url: String) -> reqwest::Response {
        let base = serve(&self.pool, caller(&self.organization, None)).await;
        reqwest::get(format!("{base}{url}")).await.unwrap()
    }
}

fn keys_of(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                found.push(key.clone());
                keys_of(inner, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys_of(item, found)),
        _ => {}
    }
}

fn checked(items: &[Value], world: &World) {
    assert_eq!(items.len(), 3);
    let read = &items[0];
    assert_eq!(read["tool"], "odoo_count");
    assert_eq!(read["access"], "read");
    assert_eq!(read["outcome"], "succeeded");
    assert_eq!(read["instance_id"], world.production.id.0.to_string());
    assert_eq!(read["instance_name"], "Odoo prod");
    assert_eq!(read["instance_environment"], "production");
    assert_eq!(read["instance_kind"], "odoo_json2");
    assert_eq!(read["actor"], INTEGRATOR_AGENT);
    assert_eq!(read["run_id"], world.run_id.0.to_string());
    assert_eq!(read["card_id"], world.card.id.0.to_string());
    assert_eq!(read["revertible"], false);

    let denied = &items[1];
    assert_eq!(denied["tool"], "odoo_write");
    assert_eq!(denied["access"], "write");
    assert_eq!(denied["outcome"], "denied");
    assert_eq!(denied["reason_code"], "write_not_granted");
    assert!(denied["reason"].is_string());
    assert_eq!(denied["instance_name"], "Odoo prod");

    let written = &items[2];
    assert_eq!(written["tool"], "odoo_write");
    assert_eq!(written["outcome"], "succeeded");
    assert_eq!(written["instance_name"], "Odoo staging");
    assert_eq!(written["instance_environment"], "staging");
    assert!(written["before"].is_array() || written["before"].is_object());
    assert!(written["after"].is_array() || written["after"].is_object());
    assert_eq!(written["revertible"], true);
    assert!(written["finished_at"].is_string());
}

#[tokio::test]
async fn a_card_journal_lists_the_actions_with_their_instance_and_no_snapshot() {
    let Some(world) = world().await else { return };
    world.journal().await;

    let response = world
        .listing(format!(
            "/api/v1/cards/{}/integration-actions",
            world.card.id
        ))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    checked(body["items"].as_array().unwrap(), &world);
    let mut keys = Vec::new();
    keys_of(&body, &mut keys);
    assert!(keys.iter().all(|key| !key.contains("snapshot")), "{keys:?}");
    assert!(!body.to_string().contains("sk-never-shown"));
    assert!(!body.to_string().contains(KEY));
}

#[tokio::test]
async fn a_run_journal_lists_the_same_actions() {
    let Some(world) = world().await else { return };
    world.journal().await;

    let response = world
        .listing(format!("/api/v1/runs/{}/integration-actions", world.run_id))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    checked(body["items"].as_array().unwrap(), &world);
}

#[tokio::test]
async fn an_instance_archived_afterwards_still_names_its_rows() {
    let Some(world) = world().await else { return };
    world.journal().await;
    world
        .usecase
        .archive_integration_instance(subject(&world.organization, true), world.staging.id)
        .await
        .unwrap();

    let response = world
        .listing(format!("/api/v1/runs/{}/integration-actions", world.run_id))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["items"][2]["instance_name"], "Odoo staging");
}

#[tokio::test]
async fn a_journal_without_actions_is_empty() {
    let Some(world) = world().await else { return };

    let response = world
        .listing(format!(
            "/api/v1/cards/{}/integration-actions",
            world.card.id
        ))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body, json!({"items": []}));
}

#[tokio::test]
async fn an_unknown_card_or_run_is_a_404() {
    let Some(world) = world().await else { return };

    let card = world
        .listing(format!(
            "/api/v1/cards/{}/integration-actions",
            CardId::new()
        ))
        .await;
    let run = world
        .listing(format!("/api/v1/runs/{}/integration-actions", RunId::new()))
        .await;

    assert_eq!(card.status(), 404);
    assert_eq!(run.status(), 404);
}

#[tokio::test]
async fn another_organization_cannot_read_the_journal() {
    let Some(world) = world().await else { return };
    world.journal().await;
    let elsewhere = organization(&world.pool).await;
    let base = serve(&world.pool, caller(&elsewhere, None)).await;

    let card = reqwest::get(format!(
        "{base}/api/v1/cards/{}/integration-actions",
        world.card.id
    ))
    .await
    .unwrap();
    let run = reqwest::get(format!(
        "{base}/api/v1/runs/{}/integration-actions",
        world.run_id
    ))
    .await
    .unwrap();

    assert_eq!(card.status(), 404);
    assert_eq!(run.status(), 404);
}

#[tokio::test]
async fn an_agent_cannot_read_the_journal() {
    let Some(world) = world().await else { return };
    world.journal().await;
    let base = serve(
        &world.pool,
        caller(&world.organization, Some(INTEGRATOR_AGENT)),
    )
    .await;

    let card = reqwest::get(format!(
        "{base}/api/v1/cards/{}/integration-actions",
        world.card.id
    ))
    .await
    .unwrap();
    let run = reqwest::get(format!(
        "{base}/api/v1/runs/{}/integration-actions",
        world.run_id
    ))
    .await
    .unwrap();

    assert_eq!(card.status(), 403);
    assert_eq!(run.status(), 403);
}

#[tokio::test]
async fn the_journal_survives_a_change_of_the_clients_of_the_card() {
    let Some(world) = world().await else { return };
    world.journal().await;
    let globex: ClientId = world
        .usecase
        .create_client(
            subject(&world.organization, false),
            CreateClient {
                name: "Globex".into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap()
        .id;
    let current = world
        .usecase
        .card_detail(subject(&world.organization, false), world.card.id)
        .await
        .unwrap()
        .card;
    world
        .usecase
        .link_card(
            subject(&world.organization, false),
            world.card.id,
            capitainerie_core::domain::card::link::LinkCard {
                expected_version: current.version,
                client: capitainerie_core::domain::client::links::Change::Set(globex),
                project: capitainerie_core::domain::client::links::Change::Keep,
            },
        )
        .await
        .unwrap();

    let response = world
        .listing(format!(
            "/api/v1/cards/{}/integration-actions",
            world.card.id
        ))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    checked(body["items"].as_array().unwrap(), &world);
}
