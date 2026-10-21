#![allow(clippy::unwrap_used)]

use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

use capitainerie_common::{CardId, CoreError, IntegrationActionId, OrganizationId, RunId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::service::CreateCard,
        client::{Client, service::CreateClient},
        conversation::CAPTAIN_AGENT,
        integration::{
            ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport,
            action::{Access, ActionCall, ActionRequest, N8nId, OdooModel},
            journal::{
                ActionContext, ActionOutcome, Grant, INTEGRATOR_AGENT, IntegrationAction, Outcome,
            },
            ports::{IntegrationActionRepository, IntegrationConnector},
            redact::MASK,
            service::CreateInstance,
        },
        run::service::SubmitTranscript,
    },
    infrastructure::{
        crypto::SecretCipher, integration::actions::PgIntegrationActionRepository, postgres,
    },
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const KEY: &str = "sk-test-123";

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

fn cipher() -> SecretCipher {
    SecretCipher::new(&[7u8; 32]).unwrap()
}

fn human(organization_id: OrganizationId, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin,
    }
}

fn agent(organization_id: OrganizationId, actor: &str) -> Subject {
    Subject {
        actor: Some(actor.into()),
        ..human(organization_id, false)
    }
}

struct Prober(&'static str);

impl IntegrationConnector for Prober {
    async fn probe(
        &self,
        _connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        Ok(ProbeReport {
            server_version: self.0.into(),
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

struct Seen {
    request: ActionRequest,
    secret: String,
    pending: i64,
    pending_with_snapshot: i64,
}

struct Scripted {
    pool: PgPool,
    answers: Mutex<HashMap<&'static str, VecDeque<Result<Value, ConnectorFailure>>>>,
    seen: Mutex<Vec<Seen>>,
}

impl Scripted {
    fn new(pool: &PgPool) -> Self {
        Self {
            pool: pool.clone(),
            answers: Mutex::new(HashMap::new()),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn answer(self, tool: &'static str, result: Result<Value, ConnectorFailure>) -> Self {
        self.answers
            .lock()
            .unwrap()
            .entry(tool)
            .or_default()
            .push_back(result);
        self
    }

    fn tools(&self) -> Vec<&'static str> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|seen| seen.request.tool())
            .collect()
    }

    fn requests(&self) -> Vec<ActionRequest> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|seen| seen.request.clone())
            .collect()
    }

    fn pending_rows(&self) -> Vec<i64> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|seen| seen.pending)
            .collect()
    }

    fn pending_rows_with_snapshot(&self) -> Vec<i64> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|seen| seen.pending_with_snapshot)
            .collect()
    }

    fn secrets(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|seen| seen.secret.clone())
            .collect()
    }
}

impl IntegrationConnector for Scripted {
    async fn probe(
        &self,
        _connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        Err(ConnectorFailure::NotSupported("not scripted".into()))
    }

    async fn perform(
        &self,
        connection: &InstanceConnection,
        request: &ActionRequest,
    ) -> Result<Value, ConnectorFailure> {
        let (pending, pending_with_snapshot): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE outcome = 'pending'),
                    count(*) FILTER (WHERE outcome = 'pending' AND snapshot_ciphertext IS NOT NULL)
             FROM integration_actions WHERE instance_id = $1",
        )
        .bind(connection.instance_id.0)
        .fetch_one(&self.pool)
        .await
        .unwrap();
        self.seen.lock().unwrap().push(Seen {
            request: request.clone(),
            secret: connection.secret.expose().to_owned(),
            pending,
            pending_with_snapshot,
        });
        self.answers
            .lock()
            .unwrap()
            .get_mut(request.tool())
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| Err(ConnectorFailure::NotSupported("not scripted".into())))
    }
}

struct World {
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization_id: OrganizationId,
    acme: Client,
    run_id: RunId,
}

async fn organization(pool: &PgPool) -> OrganizationId {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(format!("test-{}", id.simple()))
    .bind(format!("http://idp.test/realms/{id}"))
    .execute(pool)
    .await
    .unwrap();
    OrganizationId(id)
}

async fn world() -> Option<World> {
    let url = database_url().or_else(|| {
        eprintln!("skipped: DATABASE_URL is unset");
        None
    })?;
    let pool = postgres::connect(&url, 4).await.unwrap();
    let organization_id = organization(&pool).await;
    let usecase =
        CapitainerieUseCase::with_local_policies(pool.clone()).with_secrets_cipher(cipher());
    let acme = client(&usecase, organization_id, "Acme").await;
    let run_id = run(&usecase, organization_id).await;
    Some(World {
        pool,
        usecase,
        organization_id,
        acme,
        run_id,
    })
}

async fn client(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    name: &str,
) -> Client {
    usecase
        .create_client(
            human(organization_id, false),
            CreateClient {
                name: name.into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap()
}

async fn run(usecase: &CapitainerieUseCase, organization_id: OrganizationId) -> RunId {
    usecase
        .submit_transcript(SubmitTranscript {
            subject: human(organization_id, false),
            title: "Point client".into(),
            body: "Vérifier l'Odoo du client.".into(),
            metadata: Default::default(),
            choice: Default::default(),
        })
        .await
        .unwrap()
        .run_id
}

async fn card(usecase: &CapitainerieUseCase, organization_id: OrganizationId) -> CardId {
    usecase
        .create_card(
            human(organization_id, false),
            CreateCard {
                title: "Vérifier les partenaires".into(),
                description: String::new(),
                provenance: None,
                run_id: None,
                attributes: Default::default(),
            },
        )
        .await
        .unwrap()
        .id
}

impl World {
    async fn instance_of(
        &self,
        client: &Client,
        name: &str,
        kind: &str,
        version: &'static str,
    ) -> IntegrationInstance {
        self.usecase
            .create_integration_instance(
                human(self.organization_id, true),
                CreateInstance {
                    client_id: client.id,
                    kind: kind.into(),
                    name: name.into(),
                    environment: "production".into(),
                    base_url: "https://odoo.acme.test/".into(),
                    database: None,
                    login: None,
                    secret: SecretToken::new(KEY),
                },
                &Prober(version),
            )
            .await
            .unwrap()
    }

    async fn odoo(&self, name: &str) -> IntegrationInstance {
        self.instance_of(&self.acme, name, "odoo_json2", "19.0")
            .await
    }

    async fn n8n(&self, name: &str, version: &'static str) -> IntegrationInstance {
        self.instance_of(&self.acme, name, "n8n", version).await
    }

    fn context(&self, grants: &[(&IntegrationInstance, Access)]) -> ActionContext {
        self.context_for(agent(self.organization_id, INTEGRATOR_AGENT), grants)
    }

    fn context_for(
        &self,
        subject: Subject,
        grants: &[(&IntegrationInstance, Access)],
    ) -> ActionContext {
        ActionContext {
            subject,
            run_id: Some(self.run_id),
            card_id: None,
            client_id: self.acme.id,
            grants: grants
                .iter()
                .map(|(instance, access)| Grant {
                    instance_id: instance.id,
                    access: *access,
                })
                .collect(),
        }
    }

    async fn perform(
        &self,
        context: &ActionContext,
        call: ActionCall,
        connector: &Scripted,
    ) -> ActionOutcome {
        self.usecase
            .perform_integration_action(context, call, connector)
            .await
            .unwrap()
    }

    async fn journal(&self) -> Vec<IntegrationAction> {
        self.usecase
            .integration_actions_of_run(human(self.organization_id, false), self.run_id)
            .await
            .unwrap()
    }

    async fn only_row(&self) -> IntegrationAction {
        let mut rows = self.journal().await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        rows.remove(0)
    }

    async fn snapshot(&self, id: IntegrationActionId) -> Option<Value> {
        let cipher = cipher();
        let organization_id = self.organization_id;
        postgres::with_tx(&self.pool, async |tx| {
            PgIntegrationActionRepository::new(&tx, Some(&cipher))
                .snapshot(organization_id, id)
                .await
        })
        .await
        .unwrap()
    }

    async fn writes(&self) -> i64 {
        self.usecase
            .count_integration_writes(self.organization_id, self.run_id)
            .await
            .unwrap()
    }
}

fn model(name: &str) -> OdooModel {
    OdooModel::try_from(name).unwrap()
}

fn object(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn odoo_write(instance: &IntegrationInstance, values: Value) -> ActionCall {
    ActionCall {
        instance: instance.id,
        request: ActionRequest::OdooWrite {
            model: model("res.partner"),
            ids: vec![7],
            values: object(values),
        },
    }
}

fn odoo_count(instance: &IntegrationInstance) -> ActionCall {
    ActionCall {
        instance: instance.id,
        request: ActionRequest::OdooCount {
            model: model("res.partner"),
            domain: vec![],
        },
    }
}

fn odoo_read(instance: &IntegrationInstance) -> ActionCall {
    ActionCall {
        instance: instance.id,
        request: ActionRequest::OdooRead {
            model: model("res.partner"),
            ids: vec![7],
            fields: vec![],
        },
    }
}

fn update_workflow(instance: &IntegrationInstance, publish: bool) -> ActionCall {
    ActionCall {
        instance: instance.id,
        request: ActionRequest::N8nUpdateWorkflow {
            workflow_id: N8nId::try_from("wf1").unwrap(),
            name: None,
            nodes: vec![json!({"name": "Start", "type": "n8n-nodes-base.manualTrigger"})],
            connections: Default::default(),
            settings: None,
            publish,
        },
    }
}

fn workflow(active: bool) -> Value {
    json!({
        "id": "wf1",
        "name": "Relance factures",
        "active": active,
        "settings": {"executionOrder": "v1"},
        "nodes": []
    })
}

#[tokio::test]
async fn a_read_is_journaled_once_and_the_model_sees_it_masked() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Read)]);
    let connector = Scripted::new(&world.pool).answer(
        "odoo_read",
        Ok(json!([{"id": 7, "name": "Acme", "api_key": "k-live-secret"}])),
    );

    let outcome = world.perform(&context, odoo_read(&prod), &connector).await;

    assert!(outcome.succeeded);
    assert!(!outcome.wrote);
    assert!(outcome.text.contains("Acme"));
    assert!(outcome.text.contains(MASK));
    assert!(!outcome.text.contains("k-live-secret"));
    assert_eq!(
        outcome.summary,
        "odoo_read res.partner on Odoo prod: succeeded"
    );
    assert_eq!(connector.tools(), ["odoo_read"]);
    assert_eq!(connector.pending_rows(), [0]);
    let row = world.only_row().await;
    assert_eq!(Some(row.id), outcome.action_id);
    assert_eq!(row.outcome, Outcome::Succeeded);
    assert_eq!(row.access, Access::Read);
    assert_eq!(row.tool, "odoo_read");
    assert_eq!(row.instance_id, prod.id);
    assert_eq!(row.subject, "user-1");
    assert_eq!(row.actor, INTEGRATOR_AGENT);
    assert_eq!(row.run_id, Some(world.run_id));
    assert!(row.latency_ms.is_some() && row.finished_at.is_some());
    assert!(row.before.is_none() && row.after.is_none());
    assert!(!row.revertible);
    assert!(world.snapshot(row.id).await.is_none());
    let decisions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions
         WHERE run_id = $1 AND action = 'integration.call' AND actor = $2 AND allowed",
    )
    .bind(world.run_id.0)
    .bind(INTEGRATOR_AGENT)
    .fetch_one(&world.pool)
    .await
    .unwrap();
    assert_eq!(decisions, 1);
}

#[tokio::test]
async fn a_failed_read_is_a_failed_row_and_a_failure_text() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Read)]);
    let connector = Scripted::new(&world.pool).answer("odoo_count", Err(ConnectorFailure::Timeout));

    let outcome = world.perform(&context, odoo_count(&prod), &connector).await;

    assert!(!outcome.succeeded && !outcome.wrote);
    assert_eq!(
        outcome.text,
        "Failed (connection_failed): the instance did not answer in time"
    );
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Failed);
    assert_eq!(row.reason_code.as_deref(), Some("connection_failed"));
    assert_eq!(world.writes().await, 0);
}

#[tokio::test]
async fn a_write_reads_then_writes_then_reads_with_the_pending_row_and_sealed_copy_in_place() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let before = json!([{"id": 7, "name": "Old", "api_key": "k-old-secret"}]);
    let after = json!([{"id": 7, "name": "Acme", "api_key": "k-new-secret"}]);
    let connector = Scripted::new(&world.pool)
        .answer("odoo_read", Ok(before.clone()))
        .answer("odoo_write", Ok(json!(true)))
        .answer("odoo_read", Ok(after));

    let outcome = world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme", "api_key": "k-new-secret"})),
            &connector,
        )
        .await;

    assert!(outcome.succeeded && outcome.wrote);
    assert_eq!(outcome.text, "true");
    assert_eq!(connector.tools(), ["odoo_read", "odoo_write", "odoo_read"]);
    assert_eq!(connector.pending_rows(), [0, 1, 1]);
    assert_eq!(connector.pending_rows_with_snapshot(), [0, 1, 1]);
    assert!(connector.secrets().iter().all(|secret| secret == KEY));
    let requests = connector.requests();
    let ActionRequest::OdooRead { fields, ids, .. } = &requests[0] else {
        panic!("{:?}", requests[0])
    };
    let mut fields = fields.clone();
    fields.sort();
    assert_eq!(fields, ["api_key", "name"]);
    assert_eq!(ids, &[7]);
    let row = world.only_row().await;
    assert_eq!(Some(row.id), outcome.action_id);
    assert_eq!(row.outcome, Outcome::Succeeded);
    assert_eq!(row.access, Access::Write);
    assert!(row.revertible);
    assert!(row.latency_ms.is_some() && row.finished_at.is_some());
    assert_eq!(row.arguments["values"]["api_key"], MASK);
    assert_eq!(row.arguments["values"]["name"], "Acme");
    let shown_before = row.before.clone().unwrap();
    let shown_after = row.after.clone().unwrap();
    assert_eq!(shown_before[0]["api_key"], MASK);
    assert_eq!(shown_before[0]["name"], "Old");
    assert_eq!(shown_after[0]["api_key"], MASK);
    assert_eq!(shown_after[0]["name"], "Acme");
    assert_eq!(world.snapshot(row.id).await, Some(before));
    let (nonce, ciphertext): (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT snapshot_nonce, snapshot_ciphertext FROM integration_actions WHERE id = $1",
    )
    .bind(row.id.0)
    .fetch_one(&world.pool)
    .await
    .unwrap();
    assert_eq!(nonce.len(), 12);
    assert!(
        !ciphertext
            .windows("k-old-secret".len())
            .any(|window| window == b"k-old-secret")
    );
    let stored: String =
        sqlx::query_scalar("SELECT row_to_json(a)::text FROM integration_actions a WHERE id = $1")
            .bind(row.id.0)
            .fetch_one(&world.pool)
            .await
            .unwrap();
    assert!(!stored.contains("k-old-secret") && !stored.contains("k-new-secret"));
    assert_eq!(world.writes().await, 1);
}

#[tokio::test]
async fn a_write_that_fails_is_a_failed_row_with_its_copy_and_no_after() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let before = json!([{"id": 7, "name": "Old"}]);
    let connector = Scripted::new(&world.pool)
        .answer("odoo_read", Ok(before.clone()))
        .answer(
            "odoo_write",
            Err(ConnectorFailure::Remote {
                status: 500,
                message: "boom".into(),
            }),
        );

    let outcome = world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme"})),
            &connector,
        )
        .await;

    assert!(!outcome.succeeded && outcome.wrote);
    assert_eq!(
        outcome.text,
        "Failed (connection_failed): the instance answered 500: boom"
    );
    assert_eq!(connector.tools(), ["odoo_read", "odoo_write"]);
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Failed);
    assert!(!row.revertible);
    assert!(row.after.is_none());
    assert_eq!(row.before, Some(before.clone()));
    assert_eq!(world.snapshot(row.id).await, Some(before));
    assert_eq!(world.writes().await, 1);
}

#[tokio::test]
async fn a_write_on_a_read_grant_is_denied_before_any_request() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let staging = world.n8n("n8n staging", "2").await;
    let context = world.context(&[(&prod, Access::Read), (&staging, Access::Write)]);
    let connector = Scripted::new(&world.pool);

    let outcome = world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme"})),
            &connector,
        )
        .await;

    assert!(!outcome.succeeded && !outcome.wrote);
    assert!(outcome.text.starts_with("Refused (write_not_granted): "));
    assert!(connector.tools().is_empty());
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Denied);
    assert_eq!(row.reason_code.as_deref(), Some("write_not_granted"));
    assert_eq!(row.access, Access::Write);
    assert_eq!(row.instance_id, prod.id);
    assert!(row.finished_at.is_some());
    assert!(world.snapshot(row.id).await.is_none());
    assert_eq!(world.writes().await, 0);
}

#[tokio::test]
async fn an_instance_the_intervention_was_not_given_is_refused_and_journaled() {
    let Some(world) = world().await else { return };
    let staging = world.odoo("Odoo staging").await;
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&staging, Access::Write)]);
    let connector = Scripted::new(&world.pool);

    let outcome = world.perform(&context, odoo_count(&prod), &connector).await;

    assert!(!outcome.succeeded);
    assert!(outcome.text.starts_with("Refused (instance_not_granted): "));
    assert!(connector.tools().is_empty());
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Denied);
    assert_eq!(row.reason_code.as_deref(), Some("instance_not_granted"));
    assert_eq!(row.instance_id, prod.id);
}

#[tokio::test]
async fn an_instance_that_does_not_exist_is_refused_without_a_row() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let connector = Scripted::new(&world.pool);
    let call = ActionCall {
        instance: capitainerie_common::IntegrationInstanceId::new(),
        request: odoo_count(&prod).request,
    };

    let outcome = world.perform(&context, call, &connector).await;

    assert!(!outcome.succeeded);
    assert!(outcome.action_id.is_none());
    assert!(outcome.text.starts_with("Refused (instance_not_granted): "));
    assert!(connector.tools().is_empty());
    assert!(world.journal().await.is_empty());
}

#[tokio::test]
async fn an_instance_of_another_client_is_refused_even_when_granted() {
    let Some(world) = world().await else { return };
    let globex = client(&world.usecase, world.organization_id, "Globex").await;
    let foreign = world
        .instance_of(&globex, "Odoo Globex", "odoo_json2", "19.0")
        .await;
    let context = world.context(&[(&foreign, Access::Write)]);
    let connector = Scripted::new(&world.pool);

    let outcome = world
        .perform(&context, odoo_count(&foreign), &connector)
        .await;

    assert!(!outcome.succeeded);
    assert!(connector.tools().is_empty());
    let row = world.only_row().await;
    assert_eq!(
        row.reason_code.as_deref(),
        Some("instance_of_another_client")
    );
}

#[tokio::test]
async fn an_archived_instance_is_refused() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    world
        .usecase
        .archive_integration_instance(human(world.organization_id, true), prod.id)
        .await
        .unwrap();
    let connector = Scripted::new(&world.pool);

    let outcome = world.perform(&context, odoo_count(&prod), &connector).await;

    assert!(!outcome.succeeded);
    assert!(connector.tools().is_empty());
    let row = world.only_row().await;
    assert_eq!(row.reason_code.as_deref(), Some("instance_archived"));
}

#[tokio::test]
async fn a_sensitive_model_and_a_tool_of_the_other_family_are_refused() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let connector = Scripted::new(&world.pool);
    let sensitive = ActionCall {
        instance: prod.id,
        request: ActionRequest::OdooSearchRead {
            model: model("ir.config_parameter"),
            domain: vec![],
            fields: vec![],
            limit: 10,
            offset: 0,
            order: None,
        },
    };

    let refused = world.perform(&context, sensitive, &connector).await;
    let other_family = world
        .perform(&context, update_workflow(&prod, true), &connector)
        .await;

    assert!(refused.text.starts_with("Refused (sensitive_model): "));
    assert!(
        other_family
            .text
            .starts_with("Refused (tool_not_for_instance): ")
    );
    assert!(connector.tools().is_empty());
    let codes: Vec<_> = world
        .journal()
        .await
        .into_iter()
        .map(|row| row.reason_code.unwrap())
        .collect();
    assert_eq!(codes, ["sensitive_model", "tool_not_for_instance"]);
}

#[tokio::test]
async fn an_actor_the_policy_refuses_is_journaled_as_policy_denied() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context_for(
        agent(world.organization_id, CAPTAIN_AGENT),
        &[(&prod, Access::Write)],
    );
    let connector = Scripted::new(&world.pool);

    let outcome = world.perform(&context, odoo_count(&prod), &connector).await;

    assert!(!outcome.succeeded);
    assert!(outcome.text.starts_with("Refused (policy_denied): "));
    assert!(connector.tools().is_empty());
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Denied);
    assert_eq!(row.reason_code.as_deref(), Some("policy_denied"));
    assert_eq!(row.actor, CAPTAIN_AGENT);
    let refused: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions
         WHERE run_id = $1 AND action = 'integration.call' AND actor = $2 AND NOT allowed",
    )
    .bind(world.run_id.0)
    .bind(CAPTAIN_AGENT)
    .fetch_one(&world.pool)
    .await
    .unwrap();
    assert_eq!(refused, 1);
}

#[tokio::test]
async fn a_failed_copy_before_journals_a_failure_and_sends_no_write() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let connector = Scripted::new(&world.pool)
        .answer(
            "odoo_read",
            Err(ConnectorFailure::Unreachable("dns".into())),
        )
        .answer("odoo_write", Ok(json!(true)));

    let outcome = world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme"})),
            &connector,
        )
        .await;

    assert!(!outcome.succeeded && !outcome.wrote);
    assert!(
        outcome.text.starts_with(
            "Failed (before_unreadable): the state before the write could not be read: "
        )
    );
    assert_eq!(connector.tools(), ["odoo_read"]);
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Failed);
    assert_eq!(world.writes().await, 0);
    assert!(row.before.is_none());
    assert!(world.snapshot(row.id).await.is_none());
}

#[tokio::test]
async fn a_masked_value_is_put_back_from_the_copy_before_in_the_write_sent() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let connector = Scripted::new(&world.pool)
        .answer(
            "odoo_read",
            Ok(json!([{"id": 7, "name": "Old", "api_key": "k-old"}])),
        )
        .answer("odoo_write", Ok(json!(true)))
        .answer(
            "odoo_read",
            Ok(json!([{"id": 7, "name": "Acme", "api_key": "k-old"}])),
        );

    let outcome = world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme", "api_key": MASK})),
            &connector,
        )
        .await;

    assert!(outcome.succeeded);
    let requests = connector.requests();
    let ActionRequest::OdooWrite { values, .. } = &requests[1] else {
        panic!("{:?}", requests[1])
    };
    assert_eq!(values["api_key"], "k-old");
    assert_eq!(values["name"], "Acme");
    let row = world.only_row().await;
    assert_eq!(row.arguments["values"]["api_key"], MASK);
}

#[tokio::test]
async fn a_masked_value_without_an_original_is_refused_and_nothing_is_written() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let connector = Scripted::new(&world.pool)
        .answer("odoo_read", Ok(json!([{"id": 7}])))
        .answer("odoo_write", Ok(json!(true)));

    let outcome = world
        .perform(
            &context,
            odoo_write(&prod, json!({"note": MASK})),
            &connector,
        )
        .await;

    assert!(!outcome.succeeded && !outcome.wrote);
    assert!(
        outcome
            .text
            .starts_with("Refused (masked_value_without_original): ")
    );
    assert_eq!(connector.tools(), ["odoo_read"]);
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Denied);
    assert_eq!(
        row.reason_code.as_deref(),
        Some("masked_value_without_original")
    );
    assert_eq!(world.writes().await, 0);
}

#[tokio::test]
async fn n8n_1x_refuses_to_change_a_published_workflow_without_publish() {
    let Some(world) = world().await else { return };
    let staging = world.n8n("n8n staging", "1").await;
    let context = world.context(&[(&staging, Access::Write)]);
    let connector = Scripted::new(&world.pool)
        .answer("n8n_get_workflow", Ok(workflow(true)))
        .answer("n8n_update_workflow", Ok(json!({"id": "wf1"})));

    let outcome = world
        .perform(&context, update_workflow(&staging, false), &connector)
        .await;

    assert!(!outcome.succeeded && !outcome.wrote);
    assert!(outcome.text.starts_with("Refused (not_supported): "));
    assert!(
        outcome
            .text
            .contains("n8n 1.x would publish this change at once")
    );
    assert_eq!(connector.tools(), ["n8n_get_workflow"]);
    let row = world.only_row().await;
    assert_eq!(row.outcome, Outcome::Denied);
    assert_eq!(row.reason_code.as_deref(), Some("not_supported"));
}

#[tokio::test]
async fn an_n8n_update_takes_the_missing_name_and_settings_from_the_copy_before() {
    let Some(world) = world().await else { return };
    let staging = world.n8n("n8n staging", "2").await;
    let context = world.context(&[(&staging, Access::Write)]);
    let connector = Scripted::new(&world.pool)
        .answer("n8n_get_workflow", Ok(workflow(true)))
        .answer("n8n_update_workflow", Ok(json!({"id": "wf1"})))
        .answer("n8n_get_workflow", Ok(workflow(true)));

    let outcome = world
        .perform(&context, update_workflow(&staging, false), &connector)
        .await;

    assert!(outcome.succeeded && outcome.wrote);
    assert_eq!(
        connector.tools(),
        [
            "n8n_get_workflow",
            "n8n_update_workflow",
            "n8n_get_workflow"
        ]
    );
    let requests = connector.requests();
    let ActionRequest::N8nUpdateWorkflow {
        name,
        settings,
        publish,
        ..
    } = &requests[1]
    else {
        panic!("{:?}", requests[1])
    };
    assert_eq!(name.as_deref(), Some("Relance factures"));
    assert_eq!(settings.as_ref().unwrap()["executionOrder"], "v1");
    assert!(!publish);
    let row = world.only_row().await;
    assert!(row.revertible);
    assert_eq!(world.snapshot(row.id).await, Some(workflow(true)));
}

#[tokio::test]
async fn the_instance_secret_appears_in_no_column_of_the_journal() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let connector = Scripted::new(&world.pool)
        .answer(
            "odoo_count",
            Err(ConnectorFailure::Remote {
                status: 401,
                message: format!("bad key {KEY}"),
            }),
        )
        .answer(
            "odoo_read",
            Ok(json!([{"id": 7, "name": format!("note {KEY}")}])),
        )
        .answer(
            "odoo_write",
            Err(ConnectorFailure::Remote {
                status: 403,
                message: format!("Authorization: bearer {KEY}"),
            }),
        );

    let read = world.perform(&context, odoo_count(&prod), &connector).await;
    let write = world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme"})),
            &connector,
        )
        .await;

    assert!(!read.text.contains(KEY) && !read.summary.contains(KEY));
    assert!(!write.text.contains(KEY) && !write.summary.contains(KEY));
    let leaking: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM integration_actions a
         WHERE a.organization_id = $1 AND position($2 in row_to_json(a)::text) > 0",
    )
    .bind(world.organization_id.0)
    .bind(KEY)
    .fetch_one(&world.pool)
    .await
    .unwrap();
    assert_eq!(leaking, 0);
    assert_eq!(world.journal().await.len(), 2);
}

#[tokio::test]
async fn writes_are_counted_unless_they_were_denied() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let reader = world.odoo("Odoo reader").await;
    let context = world.context(&[(&prod, Access::Write), (&reader, Access::Read)]);
    let connector = Scripted::new(&world.pool)
        .answer("odoo_read", Ok(json!([{"id": 7, "name": "Old"}])))
        .answer("odoo_write", Ok(json!(true)))
        .answer("odoo_read", Ok(json!([{"id": 7, "name": "Acme"}])))
        .answer("odoo_read", Ok(json!([{"id": 7, "name": "Old"}])))
        .answer("odoo_write", Err(ConnectorFailure::Timeout));

    world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme"})),
            &connector,
        )
        .await;
    assert_eq!(world.writes().await, 1);
    world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "Acme"})),
            &connector,
        )
        .await;
    assert_eq!(world.writes().await, 2);
    world
        .perform(
            &context,
            odoo_write(&reader, json!({"name": "Acme"})),
            &connector,
        )
        .await;
    world.perform(&context, odoo_count(&prod), &connector).await;
    assert_eq!(world.writes().await, 2);
    let journal = world.journal().await;
    assert_eq!(journal.len(), 4);

    sqlx::query(
        "INSERT INTO integration_actions
            (id, organization_id, run_id, instance_id, subject, actor, tool, access, target,
             arguments, outcome, created_at)
         VALUES ($1, $2, $3, $4, 'user-1', $5, 'odoo_write', 'write', '{}', '{}', 'pending', now())",
    )
    .bind(Uuid::now_v7())
    .bind(world.organization_id.0)
    .bind(world.run_id.0)
    .bind(prod.id.0)
    .bind(INTEGRATOR_AGENT)
    .execute(&world.pool)
    .await
    .unwrap();

    assert_eq!(world.writes().await, 3);
    let elsewhere = world
        .usecase
        .count_integration_writes(world.organization_id, RunId::new())
        .await
        .unwrap();
    assert_eq!(elsewhere, 0);
    let other_organization = world
        .usecase
        .count_integration_writes(OrganizationId::new(), world.run_id)
        .await
        .unwrap();
    assert_eq!(other_organization, 0);
}

#[tokio::test]
async fn the_journal_is_listed_by_run_and_by_card_for_the_organization_only() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let card_id = card(&world.usecase, world.organization_id).await;
    let mut context = world.context(&[(&prod, Access::Read)]);
    context.card_id = Some(card_id);
    let connector = Scripted::new(&world.pool)
        .answer("odoo_count", Ok(json!(1)))
        .answer("odoo_count", Ok(json!(2)));
    world.perform(&context, odoo_count(&prod), &connector).await;
    world.perform(&context, odoo_count(&prod), &connector).await;
    let other = organization(&world.pool).await;

    let by_run = world.journal().await;
    let by_card = world
        .usecase
        .integration_actions_of_card(human(world.organization_id, false), card_id)
        .await
        .unwrap();
    let foreign_run = world
        .usecase
        .integration_actions_of_run(human(other, false), world.run_id)
        .await;
    let foreign_card = world
        .usecase
        .integration_actions_of_card(human(other, false), card_id)
        .await;
    let unknown_card = world
        .usecase
        .integration_actions_of_card(human(world.organization_id, false), CardId::new())
        .await;

    assert_eq!(by_run.len(), 2);
    assert_eq!(by_run, by_card);
    assert!(by_run[0].created_at <= by_run[1].created_at);
    assert_eq!(by_run[0].card_id, Some(card_id));
    assert!(matches!(foreign_run, Err(CoreError::NotFound)));
    assert!(matches!(foreign_card, Err(CoreError::NotFound)));
    assert!(matches!(unknown_card, Err(CoreError::NotFound)));
}

#[tokio::test]
async fn a_sealed_copy_is_bound_to_its_own_action() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let connector = Scripted::new(&world.pool)
        .answer("odoo_read", Ok(json!([{"id": 7, "name": "One"}])))
        .answer("odoo_write", Ok(json!(true)))
        .answer("odoo_read", Ok(json!([])))
        .answer("odoo_read", Ok(json!([{"id": 7, "name": "Two"}])))
        .answer("odoo_write", Ok(json!(true)))
        .answer("odoo_read", Ok(json!([])));
    world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "A"})),
            &connector,
        )
        .await;
    world
        .perform(
            &context,
            odoo_write(&prod, json!({"name": "B"})),
            &connector,
        )
        .await;
    let rows = world.journal().await;
    let (first, second) = (&rows[0], &rows[1]);

    sqlx::query(
        "UPDATE integration_actions
         SET snapshot_nonce = src.snapshot_nonce, snapshot_ciphertext = src.snapshot_ciphertext
         FROM integration_actions src
         WHERE integration_actions.id = $1 AND src.id = $2",
    )
    .bind(second.id.0)
    .bind(first.id.0)
    .execute(&world.pool)
    .await
    .unwrap();

    assert_eq!(
        world.snapshot(first.id).await,
        Some(json!([{"id": 7, "name": "One"}]))
    );
    let cipher = cipher();
    let organization_id = world.organization_id;
    let id = second.id;
    let swapped = postgres::with_tx(&world.pool, async |tx| {
        PgIntegrationActionRepository::new(&tx, Some(&cipher))
            .snapshot(organization_id, id)
            .await
    })
    .await;
    assert!(matches!(swapped, Err(CoreError::Internal(_))));
    let foreign = postgres::with_tx(&world.pool, async |tx| {
        PgIntegrationActionRepository::new(&tx, Some(&cipher))
            .snapshot(OrganizationId::new(), id)
            .await
    })
    .await;
    assert!(matches!(foreign, Ok(None)));
}

#[tokio::test]
async fn an_action_is_marked_reverted_once() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Read)]);
    let connector = Scripted::new(&world.pool)
        .answer("odoo_count", Ok(json!(1)))
        .answer("odoo_count", Ok(json!(2)));
    world.perform(&context, odoo_count(&prod), &connector).await;
    world.perform(&context, odoo_count(&prod), &connector).await;
    let rows = world.journal().await;
    let (original, revert) = (rows[0].id, rows[1].id);
    let organization_id = world.organization_id;

    let marked = postgres::with_tx(&world.pool, async |tx| {
        let mut actions = PgIntegrationActionRepository::new(&tx, None);
        let first = actions
            .mark_reverted(organization_id, original, revert)
            .await?;
        let again = actions
            .mark_reverted(organization_id, original, revert)
            .await?;
        let foreign = actions
            .mark_reverted(OrganizationId::new(), revert, original)
            .await?;
        let found = actions.find(organization_id, original).await?;
        Ok((first, again, foreign, found))
    })
    .await
    .unwrap();

    assert!(marked.0);
    assert!(!marked.1);
    assert!(!marked.2);
    assert_eq!(marked.3.unwrap().reverted_by_action_id, Some(revert));
}

#[tokio::test]
async fn without_a_secrets_key_a_call_fails_before_any_request_or_row() {
    let Some(world) = world().await else { return };
    let prod = world.odoo("Odoo prod").await;
    let context = world.context(&[(&prod, Access::Write)]);
    let keyless = CapitainerieUseCase::with_local_policies(world.pool.clone());
    let connector = Scripted::new(&world.pool);

    let outcome = keyless
        .perform_integration_action(&context, odoo_count(&prod), &connector)
        .await;
    let denied = keyless
        .perform_integration_action(&world.context(&[]), odoo_count(&prod), &connector)
        .await
        .unwrap();

    assert!(
        matches!(outcome, Err(CoreError::Internal(ref message)) if message.contains("SECRETS_KEY"))
    );
    assert!(connector.tools().is_empty());
    assert!(!denied.succeeded);
    assert_eq!(world.journal().await.len(), 1);
}
