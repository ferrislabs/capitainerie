#![allow(clippy::unwrap_used)]

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use capitainerie_auth::{Actor, Identity};
use capitainerie_common::{IntegrationActionId, OrganizationId, RunId};
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
            action::{Access, ActionCall, ActionRequest, N8nId},
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
use serde_json::{Map, Value, json};
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

fn open_connector() -> HttpConnector {
    HttpConnector::new(NetworkPolicy {
        allow_insecure_http: true,
        allow_private_networks: true,
    })
    .unwrap()
}

struct Prober;

impl IntegrationConnector for Prober {
    async fn probe(
        &self,
        _connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        Ok(ProbeReport {
            server_version: "2".into(),
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

#[derive(Clone)]
struct FakeN8n {
    workflow: Arc<Mutex<Value>>,
    calls: Arc<Mutex<Vec<String>>>,
    down: Arc<AtomicBool>,
    edits: Arc<AtomicU32>,
}

async fn read_workflow(State(fake): State<FakeN8n>, Path(id): Path<String>) -> Response {
    fake.calls.lock().unwrap().push(format!("GET {id}"));
    if fake.down.load(Ordering::SeqCst) {
        return (StatusCode::INTERNAL_SERVER_ERROR, "the fake is down").into_response();
    }
    Json(fake.workflow.lock().unwrap().clone()).into_response()
}

async fn replace_workflow(
    State(fake): State<FakeN8n>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> Response {
    let publish = query.get("publishIfActive").map(String::as_str) == Some("true");
    fake.calls
        .lock()
        .unwrap()
        .push(format!("PUT {id} publishIfActive={publish}"));
    let edit = fake.edits.fetch_add(1, Ordering::SeqCst) + 1;
    let mut workflow = fake.workflow.lock().unwrap();
    for key in ["name", "nodes", "connections", "settings"] {
        workflow[key] = body[key].clone();
    }
    workflow["updatedAt"] = json!(format!("2026-10-01T10:00:{edit:02}.000Z"));
    workflow["versionId"] = json!(format!("v{}", edit + 1));
    Json(workflow.clone()).into_response()
}

fn old_nodes() -> Value {
    json!([{"name": "Start", "type": "n8n-nodes-base.manualTrigger"}])
}

fn new_nodes() -> Value {
    json!([{"name": "Start", "type": "n8n-nodes-base.manualTrigger"}, {"name": "Mail"}])
}

async fn fake_n8n() -> (String, FakeN8n) {
    let fake = FakeN8n {
        workflow: Arc::new(Mutex::new(json!({
            "id": "wf1",
            "name": "Relance factures",
            "active": true,
            "nodes": old_nodes(),
            "connections": {"Start": {"main": []}},
            "settings": {"executionOrder": "v1"},
            "updatedAt": "2026-10-01T09:00:00.000Z",
            "versionId": "v1"
        }))),
        calls: Arc::default(),
        down: Arc::default(),
        edits: Arc::default(),
    };
    let app = Router::new()
        .route(
            "/api/v1/workflows/{id}",
            get(read_workflow).put(replace_workflow),
        )
        .with_state(fake.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    (format!("http://{address}"), fake)
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
    let app: Router =
        capitainerie_handlers_integrations::router::<()>(usecase, github, open_connector())
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
    fake: FakeN8n,
}

async fn instance(
    usecase: &CapitainerieUseCase,
    organization: &Organization,
    client: &Client,
    base_url: &str,
    name: (&str, &str),
) -> IntegrationInstance {
    usecase
        .create_integration_instance(
            subject(organization, true),
            CreateInstance {
                client_id: client.id,
                kind: "n8n".into(),
                name: name.0.into(),
                environment: name.1.into(),
                base_url: base_url.into(),
                database: None,
                login: None,
                secret: SecretToken::new(KEY),
            },
            &Prober,
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
    let (base_url, fake) = fake_n8n().await;
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
        &usecase,
        &organization,
        &client,
        &base_url,
        ("n8n staging", "staging"),
    )
    .await;
    let production = instance(
        &usecase,
        &organization,
        &client,
        &base_url,
        ("n8n prod", "production"),
    )
    .await;
    let proposed = usecase
        .create_card_linked(
            subject(&organization, false),
            CreateCard {
                title: "Mettre à jour la relance".into(),
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
                grants: vec![(staging.id, Access::Write), (production.id, Access::Write)],
                confirm_production_writes: true,
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
        fake,
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
            grants: [&self.staging, &self.production]
                .iter()
                .map(|instance| Grant {
                    instance_id: instance.id,
                    access: Access::Write,
                })
                .collect(),
            reverts_action_id: None,
        }
    }

    async fn perform(
        &self,
        instance: &IntegrationInstance,
        request: ActionRequest,
    ) -> IntegrationActionId {
        self.usecase
            .perform_integration_action(
                &self.context(),
                ActionCall {
                    instance: instance.id,
                    request,
                },
                &open_connector(),
            )
            .await
            .unwrap()
            .action_id
            .unwrap()
    }

    async fn update(&self, instance: &IntegrationInstance) -> IntegrationActionId {
        self.perform(
            instance,
            ActionRequest::N8nUpdateWorkflow {
                workflow_id: N8nId::try_from("wf1").unwrap(),
                name: None,
                nodes: new_nodes().as_array().unwrap().clone(),
                connections: Map::new(),
                settings: None,
                publish: true,
            },
        )
        .await
    }

    async fn end_run(&self) {
        sqlx::query("UPDATE runs SET status = 'succeeded' WHERE id = $1")
            .bind(self.run_id.0)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    fn calls(&self) -> Vec<String> {
        self.fake.calls.lock().unwrap().clone()
    }

    fn nodes(&self) -> Value {
        self.fake.workflow.lock().unwrap()["nodes"].clone()
    }

    async fn restore(&self, id: IntegrationActionId, body: Value) -> reqwest::Response {
        self.restore_as(caller(&self.organization, None), &id.to_string(), body)
            .await
    }

    async fn restore_as(&self, caller: Caller, id: &str, body: Value) -> reqwest::Response {
        let base = serve(&self.pool, caller).await;
        reqwest::Client::new()
            .post(format!("{base}/api/v1/integration-actions/{id}/revert"))
            .json(&body)
            .send()
            .await
            .unwrap()
    }
}

async fn refused(response: reqwest::Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status(), status);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], code, "{body}");
    assert_eq!(body["status"], status);
    assert!(body["message"].is_string());
    assert!(!body.to_string().contains(KEY));
    body
}

#[tokio::test]
async fn restoring_returns_the_restoration_rows_and_puts_the_old_workflow_back() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.staging).await;
    world.end_run().await;
    assert_eq!(world.nodes(), new_nodes());
    let sent = world.calls().len();

    let response = world.restore(original, json!({})).await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let row = &items[0];
    assert_eq!(row["tool"], "n8n_update_workflow");
    assert_eq!(row["outcome"], "succeeded");
    assert_eq!(row["access"], "write");
    assert_eq!(row["reverts_action_id"], original.0.to_string());
    assert_eq!(row["reverted_by_action_id"], Value::Null);
    assert_eq!(row["revertible"], false);
    assert_eq!(row["instance_id"], world.staging.id.0.to_string());
    assert_eq!(row["instance_name"], "n8n staging");
    assert_eq!(row["instance_environment"], "staging");
    assert_eq!(row["instance_kind"], "n8n");
    assert_eq!(row["run_id"], world.run_id.0.to_string());
    assert_eq!(row["card_id"], world.card.id.0.to_string());
    assert_eq!(row["actor"], "");
    assert_eq!(world.nodes(), old_nodes());
    assert_eq!(
        world.calls()[sent..],
        [
            "GET wf1",
            "GET wf1",
            "PUT wf1 publishIfActive=true",
            "GET wf1"
        ]
    );
    assert!(!body.to_string().contains(KEY));
    let journal = world
        .usecase
        .integration_actions_of_run(subject(&world.organization, false), world.run_id)
        .await
        .unwrap();
    let marked = journal.iter().find(|row| row.id == original).unwrap();
    assert_eq!(
        marked.reverted_by_action_id.map(|id| id.0.to_string()),
        row["id"].as_str().map(str::to_owned)
    );
}

#[tokio::test]
async fn a_workflow_edited_elsewhere_is_a_409_changed_since_and_nothing_is_written() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.staging).await;
    world.end_run().await;
    world.fake.workflow.lock().unwrap()["updatedAt"] = json!("2026-10-02T08:00:00.000Z");
    let sent = world.calls().len();

    let response = world.restore(original, json!({})).await;

    refused(response, 409, "changed_since").await;
    assert_eq!(world.calls()[sent..], ["GET wf1"]);
    assert_eq!(world.nodes(), new_nodes());
}

#[tokio::test]
async fn a_second_restoration_is_a_409_already_reverted() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.staging).await;
    world.end_run().await;
    assert_eq!(world.restore(original, json!({})).await.status(), 200);
    let sent = world.calls().len();

    let response = world.restore(original, json!({})).await;

    refused(response, 409, "already_reverted").await;
    assert_eq!(world.calls().len(), sent);
}

#[tokio::test]
async fn a_running_intervention_is_a_409_intervention_running() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.staging).await;
    let sent = world.calls().len();

    let response = world.restore(original, json!({})).await;

    refused(response, 409, "intervention_running").await;
    assert_eq!(world.calls().len(), sent);
}

#[tokio::test]
async fn an_unrestorable_action_an_archived_instance_and_an_unconfirmed_production_are_422() {
    let Some(world) = world().await else { return };
    let read = world
        .perform(
            &world.staging,
            ActionRequest::N8nGetWorkflow {
                workflow_id: N8nId::try_from("wf1").unwrap(),
            },
        )
        .await;
    let on_staging = world.update(&world.staging).await;
    let on_production = world.update(&world.production).await;
    world.end_run().await;
    let sent = world.calls().len();

    let not_revertible = world.restore(read, json!({})).await;
    let unconfirmed = world.restore(on_production, json!({})).await;
    let explicitly_unconfirmed = world
        .restore(on_production, json!({"confirm_production": false}))
        .await;
    world
        .usecase
        .archive_integration_instance(subject(&world.organization, true), world.staging.id)
        .await
        .unwrap();
    let archived = world.restore(on_staging, json!({})).await;

    refused(not_revertible, 422, "not_revertible").await;
    refused(unconfirmed, 422, "production_write_not_confirmed").await;
    refused(
        explicitly_unconfirmed,
        422,
        "production_write_not_confirmed",
    )
    .await;
    refused(archived, 422, "instance_archived").await;
    assert_eq!(world.calls().len(), sent);
}

#[tokio::test]
async fn a_production_restoration_goes_through_with_the_confirmation() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.production).await;
    world.end_run().await;

    let response = world
        .restore(original, json!({"confirm_production": true}))
        .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["items"][0]["instance_environment"], "production");
    assert_eq!(world.nodes(), old_nodes());
}

#[tokio::test]
async fn an_unreachable_instance_is_a_422_connection_failed_and_nothing_is_written() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.staging).await;
    world.end_run().await;
    world.fake.down.store(true, Ordering::SeqCst);
    let sent = world.calls().len();

    let response = world.restore(original, json!({})).await;

    refused(response, 422, "connection_failed").await;
    assert_eq!(world.calls()[sent..], ["GET wf1"]);
    assert_eq!(world.nodes(), new_nodes());
    let journal = world
        .usecase
        .integration_actions_of_run(subject(&world.organization, false), world.run_id)
        .await
        .unwrap();
    assert_eq!(journal.len(), 1);
}

#[tokio::test]
async fn an_agent_cannot_restore_and_an_unknown_action_is_a_404() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.staging).await;
    world.end_run().await;
    let sent = world.calls().len();

    let agent = world
        .restore_as(
            caller(&world.organization, Some(INTEGRATOR_AGENT)),
            &original.to_string(),
            json!({}),
        )
        .await;
    let unknown = world.restore(IntegrationActionId::new(), json!({})).await;
    let elsewhere = organization(&world.pool).await;
    let foreign = world
        .restore_as(caller(&elsewhere, None), &original.to_string(), json!({}))
        .await;

    refused(agent, 403, "forbidden").await;
    refused(unknown, 404, "not_found").await;
    refused(foreign, 404, "not_found").await;
    assert_eq!(world.calls().len(), sent);
}

#[tokio::test]
async fn a_body_that_is_not_a_json_object_is_refused() {
    let Some(world) = world().await else { return };
    let original = world.update(&world.staging).await;
    world.end_run().await;

    let wrong_type = world
        .restore(original, json!({"confirm_production": "yes"}))
        .await;
    let malformed = world
        .restore_as(caller(&world.organization, None), "not-a-uuid", json!({}))
        .await;

    assert!(wrong_type.status().is_client_error());
    assert!(malformed.status().is_client_error());
    assert_eq!(world.nodes(), new_nodes());
}
