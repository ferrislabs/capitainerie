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
        integration::{
            ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport,
            action::{Access, ActionCall, ActionRequest, N8nId, OdooModel},
            journal::{ActionContext, Grant, INTEGRATOR_AGENT, IntegrationAction, Outcome},
            ports::IntegrationConnector,
            redact::MASK,
            revert::{RevertError, RevertRefusal},
            service::CreateInstance,
        },
        run::service::SubmitTranscript,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use serde_json::{Map, Value, json};
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

fn human(organization_id: OrganizationId) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin: false,
    }
}

fn admin(organization_id: OrganizationId) -> Subject {
    Subject {
        is_admin: true,
        ..human(organization_id)
    }
}

fn integrator(organization_id: OrganizationId) -> Subject {
    Subject {
        actor: Some(INTEGRATOR_AGENT.into()),
        ..human(organization_id)
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

type Answers = HashMap<&'static str, VecDeque<Result<Value, ConnectorFailure>>>;

struct Scripted {
    answers: Mutex<Answers>,
    seen: Mutex<Vec<ActionRequest>>,
}

impl Scripted {
    fn new() -> Self {
        Self {
            answers: Mutex::new(HashMap::new()),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn answer(self, tool: &'static str, result: Value) -> Self {
        self.queue(tool, Ok(result))
    }

    fn fail(self, tool: &'static str, failure: ConnectorFailure) -> Self {
        self.queue(tool, Err(failure))
    }

    fn queue(self, tool: &'static str, result: Result<Value, ConnectorFailure>) -> Self {
        self.answers
            .lock()
            .unwrap()
            .entry(tool)
            .or_default()
            .push_back(result);
        self
    }

    fn requests(&self) -> Vec<ActionRequest> {
        self.seen.lock().unwrap().clone()
    }

    fn since(&self, mark: usize) -> Vec<ActionRequest> {
        self.requests().split_off(mark)
    }

    fn tools_since(&self, mark: usize) -> Vec<&'static str> {
        self.since(mark).iter().map(ActionRequest::tool).collect()
    }

    fn mark(&self) -> usize {
        self.seen.lock().unwrap().len()
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
        _connection: &InstanceConnection,
        request: &ActionRequest,
    ) -> Result<Value, ConnectorFailure> {
        self.seen.lock().unwrap().push(request.clone());
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
    card_id: CardId,
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
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[7u8; 32]).unwrap());
    let acme = usecase
        .create_client(
            human(organization_id),
            CreateClient {
                name: "Acme".into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap();
    let run_id = usecase
        .submit_transcript(SubmitTranscript {
            subject: human(organization_id),
            title: "Point client".into(),
            body: "Vérifier l'Odoo du client.".into(),
            metadata: Default::default(),
            choice: Default::default(),
        })
        .await
        .unwrap()
        .run_id;
    let card_id = usecase
        .create_card(
            human(organization_id),
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
        .id;
    Some(World {
        pool,
        usecase,
        organization_id,
        acme,
        run_id,
        card_id,
    })
}

impl World {
    async fn instance(
        &self,
        name: &str,
        kind: &str,
        environment: &str,
        version: &'static str,
    ) -> IntegrationInstance {
        self.usecase
            .create_integration_instance(
                admin(self.organization_id),
                CreateInstance {
                    client_id: self.acme.id,
                    kind: kind.into(),
                    name: name.into(),
                    environment: environment.into(),
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

    async fn odoo(&self) -> IntegrationInstance {
        self.instance("Odoo staging", "odoo_json2", "staging", "19.0")
            .await
    }

    async fn n8n(&self) -> IntegrationInstance {
        self.instance("n8n staging", "n8n", "staging", "2").await
    }

    fn context(&self, instance: &IntegrationInstance, access: Access) -> ActionContext {
        ActionContext {
            subject: integrator(self.organization_id),
            run_id: Some(self.run_id),
            card_id: Some(self.card_id),
            client_id: self.acme.id,
            grants: vec![Grant {
                instance_id: instance.id,
                access,
            }],
            reverts_action_id: None,
        }
    }

    async fn end_run(&self, status: &str) {
        sqlx::query("UPDATE runs SET status = $2 WHERE id = $1")
            .bind(self.run_id.0)
            .bind(status)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn act(
        &self,
        instance: &IntegrationInstance,
        request: ActionRequest,
        connector: &Scripted,
    ) -> IntegrationActionId {
        let outcome = self
            .usecase
            .perform_integration_action(
                &self.context(instance, Access::Write),
                ActionCall {
                    instance: instance.id,
                    request,
                },
                connector,
            )
            .await
            .unwrap();
        outcome.action_id.unwrap()
    }

    async fn revert(
        &self,
        id: IntegrationActionId,
        confirm_production: bool,
        connector: &Scripted,
    ) -> Result<Vec<IntegrationAction>, RevertError> {
        self.usecase
            .revert_integration_action(
                human(self.organization_id),
                id,
                confirm_production,
                connector,
            )
            .await
    }

    async fn restore(
        &self,
        id: IntegrationActionId,
        connector: &Scripted,
    ) -> Vec<IntegrationAction> {
        self.revert(id, false, connector).await.unwrap()
    }

    async fn journal(&self) -> Vec<IntegrationAction> {
        self.usecase
            .integration_actions_of_run(human(self.organization_id), self.run_id)
            .await
            .unwrap()
    }

    async fn row(&self, id: IntegrationActionId) -> IntegrationAction {
        self.journal()
            .await
            .into_iter()
            .find(|row| row.id == id)
            .unwrap()
    }
}

fn refusal(result: Result<Vec<IntegrationAction>, RevertError>) -> RevertRefusal {
    match result {
        Err(RevertError::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn model(name: &str) -> OdooModel {
    OdooModel::try_from(name).unwrap()
}

fn id(raw: &str) -> N8nId {
    N8nId::try_from(raw).unwrap()
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn odoo_write(ids: &[i64], values: Value) -> ActionRequest {
    ActionRequest::OdooWrite {
        model: model("res.partner"),
        ids: ids.to_vec(),
        values: object(values),
    }
}

fn update_workflow(nodes: Value, publish: bool) -> ActionRequest {
    ActionRequest::N8nUpdateWorkflow {
        workflow_id: id("wf1"),
        name: None,
        nodes: nodes.as_array().unwrap().clone(),
        connections: Map::new(),
        settings: None,
        publish,
    }
}

fn workflow(nodes: Value, active: bool, updated_at: &str, version: &str) -> Value {
    json!({
        "id": "wf1",
        "name": "Relance factures",
        "active": active,
        "nodes": nodes,
        "connections": {"Start": {"main": []}},
        "settings": {"executionOrder": "v1"},
        "updatedAt": updated_at,
        "versionId": version
    })
}

fn old_nodes() -> Value {
    json!([{"name": "Start", "type": "n8n-nodes-base.manualTrigger"}])
}

fn new_nodes() -> Value {
    json!([{"name": "Start", "type": "n8n-nodes-base.manualTrigger"}, {"name": "Mail"}])
}

const T0: &str = "2026-10-01T10:00:00.000Z";
const T1: &str = "2026-10-01T11:00:00.000Z";
const T2: &str = "2026-10-01T12:00:00.000Z";

struct Updated {
    action: IntegrationActionId,
    connector: Scripted,
    mark: usize,
}

async fn updated_workflow(world: &World, n8n: &IntegrationInstance) -> Updated {
    let before = workflow(old_nodes(), true, T0, "v1");
    let after = workflow(new_nodes(), true, T1, "v2");
    let connector = Scripted::new()
        .answer("n8n_get_workflow", before)
        .answer("n8n_update_workflow", after.clone())
        .answer("n8n_get_workflow", after);
    let action = world
        .act(n8n, update_workflow(new_nodes(), true), &connector)
        .await;
    let mark = connector.mark();
    Updated {
        action,
        connector,
        mark,
    }
}

#[tokio::test]
async fn restoring_an_n8n_update_sends_the_old_workflow_and_links_both_rows() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    let after = workflow(new_nodes(), true, T1, "v2");
    let restored_state = workflow(old_nodes(), true, T2, "v3");
    let connector = connector
        .answer("n8n_get_workflow", after.clone())
        .answer("n8n_get_workflow", after)
        .answer("n8n_update_workflow", restored_state.clone())
        .answer("n8n_get_workflow", restored_state.clone());
    world.end_run("succeeded").await;
    let original = world.row(action).await;
    assert!(original.revertible);
    assert!(original.reverted_by_action_id.is_none());

    let rows = world.restore(action, &connector).await;

    assert_eq!(
        connector.tools_since(mark),
        [
            "n8n_get_workflow",
            "n8n_get_workflow",
            "n8n_update_workflow",
            "n8n_get_workflow"
        ]
    );
    assert_eq!(
        connector.since(mark)[2],
        ActionRequest::N8nUpdateWorkflow {
            workflow_id: id("wf1"),
            name: Some("Relance factures".into()),
            nodes: old_nodes().as_array().unwrap().clone(),
            connections: object(json!({"Start": {"main": []}})),
            settings: Some(object(json!({"executionOrder": "v1"}))),
            publish: true,
        }
    );
    assert_eq!(rows.len(), 1);
    let restore = &rows[0];
    assert_eq!(restore.tool, "n8n_update_workflow");
    assert_eq!(restore.outcome, Outcome::Succeeded);
    assert_eq!(restore.reverts_action_id, Some(action));
    assert!(restore.reverted_by_action_id.is_none());
    assert!(!restore.revertible);
    assert_eq!(restore.instance_id, n8n.id);
    assert_eq!(restore.run_id, Some(world.run_id));
    assert_eq!(restore.card_id, Some(world.card_id));
    assert_eq!(restore.subject, "user-1");
    assert_eq!(restore.actor, "");
    assert_eq!(restore.access, Access::Write);
    assert_eq!(restore.before.as_ref().unwrap()["updatedAt"], T1);
    assert_eq!(restore.after, Some(restored_state));
    let marked = world.row(action).await;
    assert_eq!(marked.reverted_by_action_id, Some(restore.id));
    assert!(marked.revertible);
    let journal = world.journal().await;
    assert_eq!(journal.len(), 2);
    assert_eq!(journal[1].id, restore.id);
}

#[tokio::test]
async fn restoring_an_odoo_write_puts_the_old_values_back_record_by_record() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let before = json!([
        {"id": 7, "name": "Acme", "parent_id": [3, "Boss"], "category_id": [1, 2]},
        {"id": 8, "name": "Beta", "parent_id": false, "category_id": []}
    ]);
    let after = json!([
        {"id": 7, "name": "Acme SA", "parent_id": [9, "Mère"], "category_id": [4]},
        {"id": 8, "name": "Acme SA", "parent_id": [9, "Mère"], "category_id": [4]}
    ]);
    let connector = Scripted::new()
        .answer("odoo_read", before.clone())
        .answer("odoo_write", json!(true))
        .answer("odoo_read", after.clone());
    let action = world
        .act(
            &odoo,
            odoo_write(
                &[7, 8],
                json!({
                    "name": "Acme SA",
                    "parent_id": 9,
                    "category_id": [[6, 0, [4]]]
                }),
            ),
            &connector,
        )
        .await;
    let mark = connector.mark();
    let connector = connector
        .answer("odoo_read", after.clone())
        .answer("odoo_read", json!([after[0]]))
        .answer("odoo_write", json!(true))
        .answer("odoo_read", json!([before[0]]))
        .answer("odoo_read", json!([after[1]]))
        .answer("odoo_write", json!(true))
        .answer("odoo_read", json!([before[1]]));
    world.end_run("succeeded").await;

    let rows = world.restore(action, &connector).await;

    assert_eq!(
        connector.tools_since(mark),
        [
            "odoo_read",
            "odoo_read",
            "odoo_write",
            "odoo_read",
            "odoo_read",
            "odoo_write",
            "odoo_read"
        ]
    );
    let requests = connector.since(mark);
    let ActionRequest::OdooRead { ids, fields, .. } = &requests[0] else {
        panic!("the check is a read")
    };
    assert_eq!(ids, &[7, 8]);
    let mut fields = fields.clone();
    fields.sort();
    assert_eq!(fields, ["category_id", "name", "parent_id"]);
    assert_eq!(
        requests[2],
        odoo_write(
            &[7],
            json!({"name": "Acme", "parent_id": 3, "category_id": [[6, 0, [1, 2]]]})
        )
    );
    assert_eq!(
        requests[5],
        odoo_write(
            &[8],
            json!({"name": "Beta", "parent_id": false, "category_id": [[6, 0, []]]})
        )
    );
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.outcome == Outcome::Succeeded
        && row.reverts_action_id == Some(action)
        && !row.revertible));
    assert_eq!(
        world.row(action).await.reverted_by_action_id,
        Some(rows[0].id)
    );
    let shown = serde_json::to_string(
        &world
            .journal()
            .await
            .iter()
            .map(|row| (&row.arguments, &row.before, &row.after))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(!shown.contains(KEY));
}

#[tokio::test]
async fn restoring_an_odoo_create_deletes_the_created_record() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let created = json!([{"id": 31, "name": "Neuf"}]);
    let connector = Scripted::new()
        .answer("odoo_create", json!(31))
        .answer("odoo_read", created.clone());
    let action = world
        .act(
            &odoo,
            ActionRequest::OdooCreate {
                model: model("res.partner"),
                values: object(json!({"name": "Neuf"})),
            },
            &connector,
        )
        .await;
    let mark = connector.mark();
    let connector = connector
        .answer(
            "odoo_read",
            json!([{"id": 31, "name": "Neuf", "phone": "+33"}]),
        )
        .answer(
            "odoo_read",
            json!([{"id": 31, "name": "Neuf", "phone": "+33"}]),
        )
        .answer("odoo_delete", json!(true));
    world.end_run("succeeded").await;

    let rows = world.restore(action, &connector).await;

    assert_eq!(
        connector.tools_since(mark),
        ["odoo_read", "odoo_read", "odoo_delete"]
    );
    assert_eq!(
        connector.since(mark)[2],
        ActionRequest::OdooDelete {
            model: model("res.partner"),
            ids: vec![31]
        }
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tool, "odoo_delete");
    assert_eq!(rows[0].reverts_action_id, Some(action));
    assert!(!rows[0].revertible);
    assert_eq!(
        world.row(action).await.reverted_by_action_id,
        Some(rows[0].id)
    );
}

#[tokio::test]
async fn restoring_a_created_tag_deletes_it_without_reading_first() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let connector = Scripted::new().answer("n8n_create_tag", json!({"id": "14", "name": "urgent"}));
    let action = world
        .act(
            &n8n,
            ActionRequest::N8nCreateTag {
                name: "urgent".into(),
            },
            &connector,
        )
        .await;
    let mark = connector.mark();
    let connector = connector.answer("n8n_delete_tag", json!({"id": "14"}));
    world.end_run("succeeded").await;

    let rows = world.restore(action, &connector).await;

    assert_eq!(
        connector.since(mark),
        [ActionRequest::N8nDeleteTag { tag_id: id("14") }]
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tool, "n8n_delete_tag");
    assert_eq!(rows[0].reverts_action_id, Some(action));
    assert_eq!(
        world.row(action).await.reverted_by_action_id,
        Some(rows[0].id)
    );
}

#[tokio::test]
async fn restoring_an_archive_unarchives_and_restoring_tags_puts_the_old_ones_back() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let connector = Scripted::new()
        .answer("n8n_get_workflow", workflow(old_nodes(), false, T0, "v1"))
        .answer(
            "n8n_archive_workflow",
            json!({"id": "wf1", "isArchived": true}),
        )
        .answer(
            "n8n_get_workflow_tags",
            json!([{"id": "1", "name": "finance"}]),
        )
        .answer("n8n_set_workflow_tags", json!([{"id": "9"}]))
        .answer(
            "n8n_get_workflow_tags",
            json!([{"id": "9", "name": "urgent"}]),
        );
    let archived = world
        .act(
            &n8n,
            ActionRequest::N8nArchiveWorkflow {
                workflow_id: id("wf1"),
            },
            &connector,
        )
        .await;
    let tagged = world
        .act(
            &n8n,
            ActionRequest::N8nSetWorkflowTags {
                workflow_id: id("wf1"),
                tag_ids: vec![id("9")],
            },
            &connector,
        )
        .await;
    let mark = connector.mark();
    let connector = connector
        .answer("n8n_get_workflow", json!({"id": "wf1", "isArchived": true}))
        .answer("n8n_get_workflow", json!({"id": "wf1", "isArchived": true}))
        .answer(
            "n8n_unarchive_workflow",
            json!({"id": "wf1", "isArchived": false}),
        )
        .answer("n8n_get_workflow", workflow(old_nodes(), false, T2, "v1"))
        .answer(
            "n8n_get_workflow_tags",
            json!([{"id": "9", "name": "urgent"}]),
        )
        .answer(
            "n8n_get_workflow_tags",
            json!([{"id": "9", "name": "urgent"}]),
        )
        .answer("n8n_set_workflow_tags", json!([{"id": "1"}]))
        .answer(
            "n8n_get_workflow_tags",
            json!([{"id": "1", "name": "finance"}]),
        );
    world.end_run("succeeded").await;

    let unarchived = world.restore(archived, &connector).await;
    let retagged = world.restore(tagged, &connector).await;

    let requests = connector.since(mark);
    assert_eq!(
        connector.tools_since(mark),
        [
            "n8n_get_workflow",
            "n8n_get_workflow",
            "n8n_unarchive_workflow",
            "n8n_get_workflow",
            "n8n_get_workflow_tags",
            "n8n_get_workflow_tags",
            "n8n_set_workflow_tags",
            "n8n_get_workflow_tags"
        ]
    );
    assert_eq!(
        requests[2],
        ActionRequest::N8nUnarchiveWorkflow {
            workflow_id: id("wf1")
        }
    );
    assert_eq!(
        requests[6],
        ActionRequest::N8nSetWorkflowTags {
            workflow_id: id("wf1"),
            tag_ids: vec![id("1")],
        }
    );
    assert_eq!(unarchived[0].tool, "n8n_unarchive_workflow");
    assert_eq!(retagged[0].tool, "n8n_set_workflow_tags");
    assert_eq!(
        world.row(archived).await.reverted_by_action_id,
        Some(unarchived[0].id)
    );
    assert_eq!(
        world.row(tagged).await.reverted_by_action_id,
        Some(retagged[0].id)
    );
}

#[tokio::test]
async fn a_workflow_modified_since_is_a_changed_since_refusal_and_nothing_is_written() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    let connector = connector.answer("n8n_get_workflow", workflow(new_nodes(), true, T2, "v3"));
    world.end_run("succeeded").await;

    let result = world.revert(action, false, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::ChangedSince);
    assert_eq!(connector.tools_since(mark), ["n8n_get_workflow"]);
    let journal = world.journal().await;
    assert_eq!(journal.len(), 1);
    assert!(journal[0].reverted_by_action_id.is_none());
    assert!(journal[0].revertible);
}

#[tokio::test]
async fn an_odoo_record_modified_since_is_a_changed_since_refusal_and_nothing_is_written() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let connector = Scripted::new()
        .answer("odoo_read", json!([{"id": 7, "name": "Acme"}]))
        .answer("odoo_write", json!(true))
        .answer("odoo_read", json!([{"id": 7, "name": "Acme SA"}]));
    let action = world
        .act(
            &odoo,
            odoo_write(&[7], json!({"name": "Acme SA"})),
            &connector,
        )
        .await;
    let mark = connector.mark();
    let connector = connector.answer("odoo_read", json!([{"id": 7, "name": "Renommé ailleurs"}]));
    world.end_run("succeeded").await;

    let result = world.revert(action, false, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::ChangedSince);
    assert_eq!(connector.tools_since(mark), ["odoo_read"]);
    assert_eq!(world.journal().await.len(), 1);
}

#[tokio::test]
async fn a_failing_check_read_is_a_connector_error_and_nothing_is_written() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    let connector = connector.fail("n8n_get_workflow", ConnectorFailure::Timeout);
    world.end_run("succeeded").await;

    let result = world.revert(action, false, &connector).await;

    assert!(matches!(
        result,
        Err(RevertError::Connector(ConnectorFailure::Timeout))
    ));
    assert_eq!(connector.tools_since(mark), ["n8n_get_workflow"]);
    assert_eq!(world.journal().await.len(), 1);
    assert!(world.row(action).await.reverted_by_action_id.is_none());
}

#[tokio::test]
async fn an_action_already_restored_is_refused_and_nothing_is_sent() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark: _,
    } = updated_workflow(&world, &n8n).await;
    let after = workflow(new_nodes(), true, T1, "v2");
    let restored_state = workflow(old_nodes(), true, T2, "v3");
    let connector = connector
        .answer("n8n_get_workflow", after.clone())
        .answer("n8n_get_workflow", after)
        .answer("n8n_update_workflow", restored_state.clone())
        .answer("n8n_get_workflow", restored_state);
    world.end_run("succeeded").await;
    world.restore(action, &connector).await;
    let mark = connector.mark();

    let result = world.revert(action, false, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::AlreadyReverted);
    assert!(connector.since(mark).is_empty());
    assert_eq!(world.journal().await.len(), 2);
}

#[tokio::test]
async fn a_running_intervention_cannot_be_restored_until_it_ends() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;

    let queued = world.revert(action, false, &connector).await;
    world.end_run("running").await;
    let running = world.revert(action, false, &connector).await;

    assert_eq!(refusal(queued), RevertRefusal::InterventionRunning);
    assert_eq!(refusal(running), RevertRefusal::InterventionRunning);
    assert!(connector.since(mark).is_empty());
    assert_eq!(world.journal().await.len(), 1);

    let after = workflow(new_nodes(), true, T1, "v2");
    let restored_state = workflow(old_nodes(), true, T2, "v3");
    let connector = connector
        .answer("n8n_get_workflow", after.clone())
        .answer("n8n_get_workflow", after)
        .answer("n8n_update_workflow", restored_state.clone())
        .answer("n8n_get_workflow", restored_state);
    world.end_run("failed").await;
    assert_eq!(world.restore(action, &connector).await.len(), 1);
}

#[tokio::test]
async fn an_archived_instance_cannot_be_restored() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    world.end_run("succeeded").await;
    world
        .usecase
        .archive_integration_instance(admin(world.organization_id), n8n.id)
        .await
        .unwrap();

    let result = world.revert(action, true, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::InstanceArchived);
    assert!(connector.since(mark).is_empty());
    assert_eq!(world.journal().await.len(), 1);
}

#[tokio::test]
async fn a_production_instance_needs_the_confirmation_to_be_restored() {
    let Some(world) = world().await else { return };
    let prod = world.instance("n8n prod", "n8n", "production", "2").await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &prod).await;
    world.end_run("succeeded").await;

    let refused = world.revert(action, false, &connector).await;

    assert_eq!(refusal(refused), RevertRefusal::ProductionWriteNotConfirmed);
    assert!(connector.since(mark).is_empty());
    assert_eq!(world.journal().await.len(), 1);

    let after = workflow(new_nodes(), true, T1, "v2");
    let restored_state = workflow(old_nodes(), true, T2, "v3");
    let connector = connector
        .answer("n8n_get_workflow", after.clone())
        .answer("n8n_get_workflow", after)
        .answer("n8n_update_workflow", restored_state.clone())
        .answer("n8n_get_workflow", restored_state);
    let rows = world.revert(action, true, &connector).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reverts_action_id, Some(action));
}

#[tokio::test]
async fn reads_failed_and_denied_writes_and_restorations_are_not_revertible() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let connector = Scripted::new()
        .answer("odoo_count", json!(3))
        .answer("odoo_read", json!([{"id": 7, "name": "Acme"}]))
        .fail(
            "odoo_write",
            ConnectorFailure::Remote {
                status: 500,
                message: "boom".into(),
            },
        );
    let read = world
        .act(
            &odoo,
            ActionRequest::OdooCount {
                model: model("res.partner"),
                domain: vec![],
            },
            &connector,
        )
        .await;
    let failed = world
        .act(
            &odoo,
            odoo_write(&[7], json!({"name": "Acme SA"})),
            &connector,
        )
        .await;
    let denied = world
        .usecase
        .perform_integration_action(
            &world.context(&odoo, Access::Read),
            ActionCall {
                instance: odoo.id,
                request: odoo_write(&[7], json!({"name": "Acme SA"})),
            },
            &connector,
        )
        .await
        .unwrap()
        .action_id
        .unwrap();
    world.end_run("succeeded").await;
    assert_eq!(world.row(failed).await.outcome, Outcome::Failed);
    assert_eq!(world.row(denied).await.outcome, Outcome::Denied);
    let mark = connector.mark();

    for id in [read, failed, denied] {
        let result = world.revert(id, false, &connector).await;
        assert_eq!(refusal(result), RevertRefusal::NotRevertible);
    }

    assert!(connector.since(mark).is_empty());
}

#[tokio::test]
async fn a_restoration_cannot_itself_be_restored() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let connector = Scripted::new().answer("n8n_create_tag", json!({"id": "14"}));
    let action = world
        .act(
            &n8n,
            ActionRequest::N8nCreateTag {
                name: "urgent".into(),
            },
            &connector,
        )
        .await;
    let connector = connector.answer("n8n_delete_tag", json!({"id": "14"}));
    world.end_run("succeeded").await;
    let rows = world.restore(action, &connector).await;
    let mark = connector.mark();

    let result = world.revert(rows[0].id, false, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::NotRevertible);
    assert!(connector.since(mark).is_empty());
    assert!(!world.row(rows[0].id).await.revertible);
}

#[tokio::test]
async fn an_action_whose_sealed_copy_is_gone_is_not_revertible() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    world.end_run("succeeded").await;
    sqlx::query(
        "UPDATE integration_actions SET snapshot_nonce = NULL, snapshot_ciphertext = NULL WHERE id = $1",
    )
    .bind(action.0)
    .execute(&world.pool)
    .await
    .unwrap();

    let result = world.revert(action, false, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::NotRevertible);
    assert!(connector.since(mark).is_empty());
}

#[tokio::test]
async fn an_agent_cannot_restore_an_action() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    world.end_run("succeeded").await;

    for actor in [INTEGRATOR_AGENT, "capitainerie-agent-writer"] {
        let subject = Subject {
            actor: Some(actor.into()),
            ..human(world.organization_id)
        };
        let result = world
            .usecase
            .revert_integration_action(subject, action, true, &connector)
            .await;
        assert!(
            matches!(result, Err(RevertError::Core(CoreError::Forbidden { .. }))),
            "{actor}: {result:?}"
        );
    }

    assert!(connector.since(mark).is_empty());
    assert_eq!(world.journal().await.len(), 1);
}

#[tokio::test]
async fn an_unknown_action_or_one_of_another_organization_is_not_found() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    world.end_run("succeeded").await;
    let elsewhere = organization(&world.pool).await;

    let unknown = world
        .revert(IntegrationActionId::new(), false, &connector)
        .await;
    let foreign = world
        .usecase
        .revert_integration_action(human(elsewhere), action, true, &connector)
        .await;

    assert!(matches!(
        unknown,
        Err(RevertError::Core(CoreError::NotFound))
    ));
    assert!(matches!(
        foreign,
        Err(RevertError::Core(CoreError::NotFound))
    ));
    assert!(connector.since(mark).is_empty());
}

#[tokio::test]
async fn a_restoration_failing_midway_keeps_its_rows_and_leaves_the_original_unrestored() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let before = json!([{"id": 7, "name": "Acme"}, {"id": 8, "name": "Beta"}]);
    let after = json!([{"id": 7, "name": "Acme SA"}, {"id": 8, "name": "Acme SA"}]);
    let connector = Scripted::new()
        .answer("odoo_read", before.clone())
        .answer("odoo_write", json!(true))
        .answer("odoo_read", after.clone());
    let action = world
        .act(
            &odoo,
            odoo_write(&[7, 8], json!({"name": "Acme SA"})),
            &connector,
        )
        .await;
    let mark = connector.mark();
    let connector = connector
        .answer("odoo_read", after.clone())
        .answer("odoo_read", json!([after[0]]))
        .answer("odoo_write", json!(true))
        .answer("odoo_read", json!([before[0]]))
        .answer("odoo_read", json!([after[1]]))
        .fail("odoo_write", ConnectorFailure::Timeout);
    world.end_run("succeeded").await;

    let rows = world.restore(action, &connector).await;

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].outcome, Outcome::Succeeded);
    assert_eq!(rows[1].outcome, Outcome::Failed);
    assert_eq!(rows[1].reason_code.as_deref(), Some("connection_failed"));
    assert!(rows.iter().all(|row| row.reverts_action_id == Some(action)));
    assert_eq!(
        connector.tools_since(mark),
        [
            "odoo_read",
            "odoo_read",
            "odoo_write",
            "odoo_read",
            "odoo_read",
            "odoo_write"
        ]
    );
    let original = world.row(action).await;
    assert!(original.reverted_by_action_id.is_none());
    assert_eq!(world.journal().await.len(), 3);
}

impl World {
    async fn claimed(&self, id: IntegrationActionId) -> bool {
        sqlx::query_scalar(
            "SELECT revert_claimed_at IS NOT NULL FROM integration_actions WHERE id = $1",
        )
        .bind(id.0)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }
}

fn restore_answers(connector: Scripted) -> Scripted {
    let after = workflow(new_nodes(), true, T1, "v2");
    let restored_state = workflow(old_nodes(), true, T2, "v3");
    connector
        .answer("n8n_get_workflow", after.clone())
        .answer("n8n_get_workflow", after)
        .answer("n8n_update_workflow", restored_state.clone())
        .answer("n8n_get_workflow", restored_state)
}

#[tokio::test]
async fn two_simultaneous_restorations_of_one_action_write_only_once() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    let connector = restore_answers(connector);
    world.end_run("succeeded").await;

    let (first, second) = tokio::join!(
        world.revert(action, false, &connector),
        world.revert(action, false, &connector)
    );

    let (restored, refused) = match (first, second) {
        (Ok(rows), Err(error)) | (Err(error), Ok(rows)) => (rows, error),
        other => panic!("exactly one restoration must go through, got {other:?}"),
    };
    assert!(matches!(
        refused,
        RevertError::Refused(RevertRefusal::AlreadyReverted)
    ));
    assert_eq!(restored.len(), 1);
    assert_eq!(
        connector
            .tools_since(mark)
            .iter()
            .filter(|tool| **tool == "n8n_update_workflow")
            .count(),
        1
    );
    assert_eq!(world.journal().await.len(), 2);
    let marked = world.row(action).await;
    assert_eq!(marked.reverted_by_action_id, Some(restored[0].id));
    assert!(!world.claimed(action).await);
}

#[tokio::test]
async fn a_reserved_action_is_refused_until_the_reservation_expires() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark,
    } = updated_workflow(&world, &n8n).await;
    world.end_run("succeeded").await;
    sqlx::query("UPDATE integration_actions SET revert_claimed_at = now() WHERE id = $1")
        .bind(action.0)
        .execute(&world.pool)
        .await
        .unwrap();

    let fresh = world.revert(action, false, &connector).await;

    assert_eq!(refusal(fresh), RevertRefusal::AlreadyReverted);
    assert!(connector.since(mark).is_empty());
    assert!(world.claimed(action).await);

    sqlx::query(
        "UPDATE integration_actions SET revert_claimed_at = now() - interval '11 minutes' WHERE id = $1",
    )
    .bind(action.0)
    .execute(&world.pool)
    .await
    .unwrap();
    let connector = restore_answers(connector);

    let rows = world.restore(action, &connector).await;

    assert_eq!(rows.len(), 1);
    assert!(!world.claimed(action).await);
    assert_eq!(
        world.row(action).await.reverted_by_action_id,
        Some(rows[0].id)
    );
}

#[tokio::test]
async fn a_refused_or_failed_restoration_gives_the_action_back() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action,
        connector,
        mark: _,
    } = updated_workflow(&world, &n8n).await;
    world.end_run("succeeded").await;
    let after = workflow(new_nodes(), true, T1, "v2");

    let connector = connector.answer("n8n_get_workflow", workflow(new_nodes(), true, T2, "v3"));
    let changed = world.revert(action, false, &connector).await;
    assert_eq!(refusal(changed), RevertRefusal::ChangedSince);
    assert!(!world.claimed(action).await);

    let connector = connector.fail("n8n_get_workflow", ConnectorFailure::Timeout);
    let unreadable = world.revert(action, false, &connector).await;
    assert!(matches!(
        unreadable,
        Err(RevertError::Connector(ConnectorFailure::Timeout))
    ));
    assert!(!world.claimed(action).await);

    let connector = connector
        .answer("n8n_get_workflow", after.clone())
        .answer("n8n_get_workflow", after.clone())
        .fail("n8n_update_workflow", ConnectorFailure::Timeout);
    let failed = world.restore(action, &connector).await;
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].outcome, Outcome::Failed);
    assert!(!world.claimed(action).await);
    assert!(world.row(action).await.reverted_by_action_id.is_none());

    let connector = connector
        .answer("n8n_get_workflow", after.clone())
        .answer("n8n_get_workflow", after)
        .answer("n8n_update_workflow", workflow(old_nodes(), true, T2, "v4"))
        .answer("n8n_get_workflow", workflow(old_nodes(), true, T2, "v4"));
    let rows = world.restore(action, &connector).await;
    assert_eq!(rows[0].outcome, Outcome::Succeeded);
    assert_eq!(
        world.row(action).await.reverted_by_action_id,
        Some(rows[0].id)
    );
    assert!(!world.claimed(action).await);
}

#[tokio::test]
async fn a_refusal_before_the_reservation_leaves_nothing_reserved() {
    let Some(world) = world().await else { return };
    let n8n = world.n8n().await;
    let Updated {
        action, connector, ..
    } = updated_workflow(&world, &n8n).await;

    let running = world.revert(action, false, &connector).await;
    let unconfirmed = {
        world.end_run("succeeded").await;
        sqlx::query("UPDATE integration_instances SET environment = 'production' WHERE id = $1")
            .bind(n8n.id.0)
            .execute(&world.pool)
            .await
            .unwrap();
        world.revert(action, false, &connector).await
    };

    assert_eq!(refusal(running), RevertRefusal::InterventionRunning);
    assert_eq!(
        refusal(unconfirmed),
        RevertRefusal::ProductionWriteNotConfirmed
    );
    assert!(!world.claimed(action).await);
}

async fn half_restored(
    world: &World,
    odoo: &IntegrationInstance,
) -> (IntegrationActionId, Scripted, Value, Value) {
    let before = json!([{"id": 7, "name": "Acme"}, {"id": 8, "name": "Beta"}]);
    let after = json!([{"id": 7, "name": "Acme SA"}, {"id": 8, "name": "Acme SA"}]);
    let connector = Scripted::new()
        .answer("odoo_read", before.clone())
        .answer("odoo_write", json!(true))
        .answer("odoo_read", after.clone());
    let action = world
        .act(
            odoo,
            odoo_write(&[7, 8], json!({"name": "Acme SA"})),
            &connector,
        )
        .await;
    let connector = connector
        .answer("odoo_read", after.clone())
        .answer("odoo_read", json!([after[0]]))
        .answer("odoo_write", json!(true))
        .answer("odoo_read", json!([before[0]]))
        .answer("odoo_read", json!([after[1]]))
        .fail("odoo_write", ConnectorFailure::Timeout);
    world.end_run("succeeded").await;
    let rows = world.restore(action, &connector).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].outcome, Outcome::Failed);
    assert!(!world.claimed(action).await);
    (action, connector, before, after)
}

#[tokio::test]
async fn an_odoo_restoration_stopped_on_its_second_record_resumes_where_it_stopped() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let (action, connector, before, after) = half_restored(&world, &odoo).await;
    let mark = connector.mark();
    let connector = connector
        .answer("odoo_read", json!([before[0], after[1]]))
        .answer("odoo_read", json!([after[1]]))
        .answer("odoo_write", json!(true))
        .answer("odoo_read", json!([before[1]]));

    let rows = world.restore(action, &connector).await;

    assert_eq!(
        connector.tools_since(mark),
        ["odoo_read", "odoo_read", "odoo_write", "odoo_read"]
    );
    assert_eq!(
        connector.since(mark)[2],
        odoo_write(&[8], json!({"name": "Beta"}))
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, Outcome::Succeeded);
    assert_eq!(rows[0].reverts_action_id, Some(action));
    assert_eq!(
        world.row(action).await.reverted_by_action_id,
        Some(rows[0].id)
    );
    assert_eq!(world.journal().await.len(), 4);
    assert!(!world.claimed(action).await);
}

#[tokio::test]
async fn an_odoo_restoration_with_every_record_already_back_writes_nothing_and_closes_the_original()
{
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let (action, connector, before, _) = half_restored(&world, &odoo).await;
    let mark = connector.mark();
    let connector = connector.answer("odoo_read", before.clone());

    let rows = world.restore(action, &connector).await;

    assert_eq!(connector.tools_since(mark), ["odoo_read"]);
    assert!(rows.is_empty());
    let journal = world.journal().await;
    assert_eq!(journal.len(), 3);
    let first_restoration = journal
        .iter()
        .find(|row| row.reverts_action_id == Some(action) && row.outcome == Outcome::Succeeded)
        .unwrap();
    assert_eq!(
        world.row(action).await.reverted_by_action_id,
        Some(first_restoration.id)
    );
    assert!(!world.claimed(action).await);
    let again = world.revert(action, false, &connector).await;
    assert_eq!(refusal(again), RevertRefusal::AlreadyReverted);
}

#[tokio::test]
async fn an_odoo_restoration_with_a_record_neither_as_written_nor_as_before_is_changed_since() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let (action, connector, before, _) = half_restored(&world, &odoo).await;
    let mark = connector.mark();
    let connector = connector.answer(
        "odoo_read",
        json!([before[0], {"id": 8, "name": "Renommé ailleurs"}]),
    );

    let result = world.revert(action, false, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::ChangedSince);
    assert_eq!(connector.tools_since(mark), ["odoo_read"]);
    assert!(!world.claimed(action).await);
    assert!(world.row(action).await.reverted_by_action_id.is_none());
}

#[tokio::test]
async fn an_odoo_write_of_a_masked_field_cannot_be_restored() {
    let Some(world) = world().await else { return };
    let odoo = world.odoo().await;
    let connector = Scripted::new()
        .answer(
            "odoo_read",
            json!([{"id": 7, "name": "Acme", "api_key": "k-old"}]),
        )
        .answer("odoo_write", json!(true))
        .answer(
            "odoo_read",
            json!([{"id": 7, "name": "Acme", "api_key": "k-new"}]),
        );
    let action = world
        .act(
            &odoo,
            odoo_write(&[7], json!({"api_key": "k-new"})),
            &connector,
        )
        .await;
    let mark = connector.mark();
    world.end_run("succeeded").await;

    let result = world.revert(action, false, &connector).await;

    assert_eq!(refusal(result), RevertRefusal::NotRevertible);
    assert!(connector.since(mark).is_empty());
    assert!(!world.claimed(action).await);
    let row = world.row(action).await;
    assert_eq!(row.arguments["values"]["api_key"], MASK);
    let shown = serde_json::to_string(&(&row.arguments, &row.before, &row.after)).unwrap();
    assert!(!shown.contains("k-old") && !shown.contains("k-new"));
}
