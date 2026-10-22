#![allow(clippy::unwrap_used)]

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use capitainerie_common::{CardId, CoreError, OrganizationId, RunId, StepId};
use capitainerie_core::{
    CapitainerieUseCase, InterventionConfig, InterventionPipeline,
    domain::{
        agent::{SecretToken, Usage},
        authorization::Subject,
        card::{
            Card, Column,
            service::{CreateCard, MoveCard},
        },
        client::{Client, service::CreateClient},
        conversation::ToolRecord,
        integration::{
            ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport,
            action::{Access, ActionRequest},
            intervention::{
                ConfideCard, INTERVENTION_DEADLINE, IntegratorReport, IntegratorRequest,
                IntegratorRuntime, IntegratorTools, ToolAnswer, intervention_limits,
            },
            journal::INTEGRATOR_AGENT,
            ports::IntegrationConnector,
            service::CreateInstance,
        },
        provider::{ModelAssignments, ModelRef, ProviderKind},
        run::{
            StepKind,
            step::{Step, StepStatus},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const KEY: &str = "sk-0123456789abcdef-supersecret-wxyz";
const INTERRUPTED: &str = "interrompue après des écritures, voir le journal";

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

fn human(organization_id: OrganizationId, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin,
    }
}

#[derive(Clone, Default)]
struct Connector {
    seen: Arc<Mutex<Vec<&'static str>>>,
}

impl Connector {
    fn tools(&self) -> Vec<&'static str> {
        self.seen.lock().unwrap().clone()
    }
}

impl IntegrationConnector for Connector {
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
        connection: &InstanceConnection,
        request: &ActionRequest,
    ) -> Result<Value, ConnectorFailure> {
        assert_eq!(connection.secret.expose(), KEY);
        self.seen.lock().unwrap().push(request.tool());
        match request {
            ActionRequest::OdooRead { .. } => Ok(json!([{"id": 7, "name": "Ancien nom"}])),
            ActionRequest::OdooWrite { .. } => Ok(json!(true)),
            ActionRequest::OdooCount { .. } => Ok(json!(3)),
            other => Err(ConnectorFailure::NotSupported(other.tool().to_owned())),
        }
    }
}

#[derive(Clone)]
enum Move {
    Call(&'static str, Value),
    AgeHeartbeat,
    ObserveHeartbeat,
}

#[derive(Clone, Default)]
struct Seen {
    runs: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<IntegratorRequest>>>,
    specs: Arc<Mutex<Vec<String>>>,
    answers: Arc<Mutex<Vec<ToolAnswer>>>,
    heartbeats: Arc<Mutex<Vec<Option<DateTime<Utc>>>>>,
}

impl Seen {
    fn runs(&self) -> usize {
        self.runs.load(Ordering::SeqCst)
    }

    fn answers(&self) -> Vec<ToolAnswer> {
        self.answers.lock().unwrap().clone()
    }

    fn specs(&self) -> Vec<String> {
        self.specs.lock().unwrap().clone()
    }

    fn request(&self) -> IntegratorRequest {
        self.requests.lock().unwrap().last().unwrap().clone()
    }
}

#[derive(Clone)]
struct Script {
    pool: PgPool,
    run_id: RunId,
    moves: Vec<Move>,
    text: String,
    stopped: Option<String>,
    failure: Option<String>,
    seen: Seen,
}

impl Script {
    fn stopped(mut self, reason: &str) -> Self {
        self.stopped = Some(reason.to_owned());
        self
    }

    fn failing(mut self, reason: &str) -> Self {
        self.failure = Some(reason.to_owned());
        self
    }
}

impl IntegratorRuntime for Script {
    async fn intervene<T: IntegratorTools>(
        &self,
        request: &IntegratorRequest,
        tools: &T,
    ) -> Result<IntegratorReport, CoreError> {
        self.seen.runs.fetch_add(1, Ordering::SeqCst);
        self.seen.requests.lock().unwrap().push(request.clone());
        *self.seen.specs.lock().unwrap() =
            tools.specs().into_iter().map(|spec| spec.name).collect();
        let mut records = Vec::new();
        for next in &self.moves {
            match next {
                Move::Call(name, arguments) => {
                    let answer = tools.call(name, arguments).await;
                    records.push(ToolRecord {
                        name: (*name).to_owned(),
                        arguments: arguments.to_string(),
                        succeeded: answer.succeeded,
                        latency_ms: 0,
                        summary: answer.summary.clone(),
                    });
                    self.seen.answers.lock().unwrap().push(answer);
                }
                Move::AgeHeartbeat => {
                    sqlx::query(
                        "UPDATE runs SET heartbeat_at = now() - interval '2 hours' WHERE id = $1",
                    )
                    .bind(self.run_id.0)
                    .execute(&self.pool)
                    .await
                    .unwrap();
                }
                Move::ObserveHeartbeat => {
                    let at: Option<DateTime<Utc>> =
                        sqlx::query_scalar("SELECT heartbeat_at FROM runs WHERE id = $1")
                            .bind(self.run_id.0)
                            .fetch_one(&self.pool)
                            .await
                            .unwrap();
                    self.seen.heartbeats.lock().unwrap().push(at);
                }
            }
        }
        if let Some(reason) = &self.failure {
            return Err(CoreError::Internal(reason.clone()));
        }
        Ok(IntegratorReport {
            text: self.text.clone(),
            tool_calls: records,
            usage: Usage {
                model: "chat".into(),
                input_tokens: 100,
                output_tokens: 20,
            },
            cost_micros: 1_234,
            stopped: self.stopped.clone(),
        })
    }
}

#[derive(Clone, Copy)]
enum Slot {
    Staging,
    Prod,
    Other,
}

struct World {
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization_id: OrganizationId,
    card: Card,
    staging: IntegrationInstance,
    prod: IntegrationInstance,
    other: IntegrationInstance,
    run_id: RunId,
}

type Pipeline = InterventionPipeline<Script, Connector>;

fn config() -> InterventionConfig {
    InterventionConfig {
        stale_after: Duration::from_secs(600),
        default_models: ModelAssignments::defaults("chat", "embed"),
        console_url: Some("https://console.test/".into()),
        deadline: INTERVENTION_DEADLINE,
    }
}

async fn instance(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    client: &Client,
    name: &str,
    environment: &str,
) -> IntegrationInstance {
    usecase
        .create_integration_instance(
            human(organization_id, true),
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
            &Connector::default(),
        )
        .await
        .unwrap()
}

async fn world(grants: &[(Slot, Access)]) -> Option<World> {
    let url = database_url().or_else(|| {
        eprintln!("skipped: DATABASE_URL is unset");
        None
    })?;
    let pool = postgres::connect(&url, 4).await.unwrap();
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(format!("test-{}", id.simple()))
    .bind(format!("http://idp.test/realms/{id}"))
    .execute(&pool)
    .await
    .unwrap();
    let organization_id = OrganizationId(id);
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::new(&[7u8; 32]).unwrap())
        .with_secrets_cipher(SecretCipher::new(&[9u8; 32]).unwrap());
    let acme = usecase
        .create_client(
            human(organization_id, false),
            CreateClient {
                name: "Acme".into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap();
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let prod = instance(&usecase, organization_id, &acme, "Odoo prod", "production").await;
    let other = instance(
        &usecase,
        organization_id,
        &acme,
        "Odoo autre",
        "development",
    )
    .await;
    let proposed = usecase
        .create_card_linked(
            human(organization_id, false),
            CreateCard {
                title: "Vérifier les factures d'avril".into(),
                description: "Dans l'Odoo du client".into(),
                provenance: None,
                run_id: None,
                attributes: Default::default(),
            },
            Some(acme.id),
            None,
        )
        .await
        .unwrap();
    let todo = usecase
        .move_card(
            human(organization_id, false),
            proposed.id,
            MoveCard {
                to: Column::Todo,
                expected_version: proposed.version,
                run_id: None,
            },
        )
        .await
        .unwrap();
    let chosen = grants
        .iter()
        .map(|(slot, access)| {
            let granted = match slot {
                Slot::Staging => &staging,
                Slot::Prod => &prod,
                Slot::Other => &other,
            };
            (granted.id, *access)
        })
        .collect();
    let started = usecase
        .confide_card_to_integrator(
            human(organization_id, false),
            todo.id,
            ConfideCard {
                instructions: "Ne touche pas aux factures payées".into(),
                grants: chosen,
                confirm_production_writes: false,
                expected_version: todo.version,
            },
        )
        .await
        .unwrap();
    Some(World {
        pool,
        usecase,
        organization_id,
        card: started.card,
        staging,
        prod,
        other,
        run_id: started.run_id,
    })
}

impl World {
    fn script(&self, seen: &Seen, moves: Vec<Move>) -> Script {
        Script {
            pool: self.pool.clone(),
            run_id: self.run_id,
            moves,
            text: "Rapport de l'Intervenant".into(),
            stopped: None,
            failure: None,
            seen: seen.clone(),
        }
    }

    fn pipeline(&self, script: Script, connector: &Connector) -> Pipeline {
        InterventionPipeline::new(self.usecase.clone(), script, connector.clone(), config())
    }

    fn call(&self, tool: &'static str, slot: Slot, extra: Value) -> Move {
        let target = match slot {
            Slot::Staging => &self.staging,
            Slot::Prod => &self.prod,
            Slot::Other => &self.other,
        };
        let mut arguments = extra;
        arguments["instance"] = json!(target.id.0.to_string());
        Move::Call(tool, arguments)
    }

    async fn run(&self, script: Script, connector: &Connector) {
        self.pipeline(script, connector)
            .execute(self.run_id, None)
            .await
            .unwrap();
    }

    async fn crashed(&self) {
        sqlx::query(
            "UPDATE runs SET status = 'running', started_at = now(), heartbeat_at = now() - interval '2 hours' WHERE id = $1",
        )
        .bind(self.run_id.0)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    async fn record_write(&self, outcome: &str, reason_code: Option<&str>) {
        sqlx::query(
            "INSERT INTO integration_actions (id, organization_id, run_id, card_id, instance_id, subject, actor, tool, access, target, arguments, outcome, reason_code, created_at)
             VALUES ($1, $2, $3, $4, $5, 'user-1', $6, 'odoo_write', 'write', '{}'::jsonb, '{}'::jsonb, $7, $8, now())",
        )
        .bind(Uuid::now_v7())
        .bind(self.organization_id.0)
        .bind(self.run_id.0)
        .bind(self.card.id.0)
        .bind(self.staging.id.0)
        .bind(INTEGRATOR_AGENT)
        .bind(outcome)
        .bind(reason_code)
        .execute(&self.pool)
        .await
        .unwrap();
    }
}

async fn run_row(pool: &PgPool, run_id: RunId) -> (String, Option<String>) {
    sqlx::query_as("SELECT status, failure FROM runs WHERE id = $1")
        .bind(run_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[derive(sqlx::FromRow)]
struct StepRow {
    status: String,
    kind: String,
    attempt: i32,
    error: Option<String>,
    cost_micros: i64,
    input_tokens: i64,
    output_tokens: i64,
    latency_ms: Option<i64>,
    output: Value,
}

async fn step(pool: &PgPool, run_id: RunId) -> StepRow {
    sqlx::query_as(
        "SELECT status, kind, attempt, error, cost_micros, input_tokens, output_tokens, latency_ms, output FROM run_steps WHERE run_id = $1",
    )
    .bind(run_id.0)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn comments(pool: &PgPool, card_id: CardId) -> Vec<(String, Option<String>, String)> {
    sqlx::query_as(
        "SELECT author_subject, author_actor, body FROM card_comments WHERE card_id = $1 ORDER BY created_at",
    )
    .bind(card_id.0)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn column(pool: &PgPool, card_id: CardId) -> String {
    sqlx::query_scalar("SELECT \"column\" FROM cards WHERE id = $1")
        .bind(card_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn journal(pool: &PgPool, run_id: RunId) -> Vec<(String, String, String, Option<String>)> {
    sqlx::query_as(
        "SELECT tool, access, outcome, reason_code FROM integration_actions WHERE run_id = $1 ORDER BY created_at, id",
    )
    .bind(run_id.0)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn row(
    tool: &str,
    access: &str,
    outcome: &str,
    reason_code: Option<&str>,
) -> (String, String, String, Option<String>) {
    (
        tool.to_owned(),
        access.to_owned(),
        outcome.to_owned(),
        reason_code.map(str::to_owned),
    )
}

#[tokio::test]
async fn list_instances_shows_only_the_granted_instance_and_another_one_is_refused() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world.script(
        &seen,
        vec![
            Move::Call("list_instances", json!({})),
            world.call("odoo_count", Slot::Prod, json!({"model": "res.partner"})),
            world.call("odoo_count", Slot::Other, json!({"model": "res.partner"})),
        ],
    );

    world.run(script, &connector).await;

    let answers = seen.answers();
    let listed: Value = serde_json::from_str(&answers[0].text).unwrap();
    assert_eq!(
        listed,
        json!([{
            "id": world.staging.id.0.to_string(),
            "name": "Odoo staging",
            "kind": "odoo_json2",
            "environment": "staging",
            "access": "write",
        }])
    );
    assert!(answers[0].succeeded);
    assert!(!answers[0].text.contains(&world.prod.id.0.to_string()));
    assert!(!answers[0].text.contains(&world.other.id.0.to_string()));
    for refused in &answers[1..] {
        assert!(!refused.succeeded);
        assert!(refused.text.starts_with("Refused (instance_not_granted)"));
    }
    assert!(connector.tools().is_empty());
    let denied = row("odoo_count", "read", "denied", Some("instance_not_granted"));
    assert_eq!(
        journal(&world.pool, world.run_id).await,
        [denied.clone(), denied]
    );
    let specs = seen.specs();
    assert_eq!(specs[0], "list_instances");
    assert!(specs.iter().any(|name| name == "odoo_write"));
    assert!(specs.iter().any(|name| name == "n8n_list_workflows"));
}

#[tokio::test]
async fn a_write_on_a_read_only_instance_is_refused_without_reaching_the_instance() {
    let Some(world) = world(&[(Slot::Prod, Access::Read), (Slot::Staging, Access::Write)]).await
    else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world.script(
        &seen,
        vec![world.call(
            "odoo_write",
            Slot::Prod,
            json!({"model": "res.partner", "ids": [7], "values": {"name": "Acme SA"}}),
        )],
    );

    world.run(script, &connector).await;

    let answers = seen.answers();
    assert!(!answers[0].succeeded);
    assert!(answers[0].text.starts_with("Refused (write_not_granted)"));
    assert!(connector.tools().is_empty(), "no call reached the instance");
    assert_eq!(
        journal(&world.pool, world.run_id).await,
        [row(
            "odoo_write",
            "write",
            "denied",
            Some("write_not_granted")
        )]
    );
    assert_eq!(
        world
            .usecase
            .count_integration_writes(world.organization_id, world.run_id)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn a_granted_write_goes_through_and_is_journaled() {
    let Some(world) = world(&[(Slot::Prod, Access::Read), (Slot::Staging, Access::Write)]).await
    else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world.script(
        &seen,
        vec![
            world.call(
                "odoo_write",
                Slot::Prod,
                json!({"model": "res.partner", "ids": [7], "values": {"name": "Acme SA"}}),
            ),
            world.call(
                "odoo_write",
                Slot::Staging,
                json!({"model": "res.partner", "ids": [7], "values": {"name": "Acme SA"}}),
            ),
        ],
    );

    world.run(script, &connector).await;

    let answers = seen.answers();
    assert!(!answers[0].succeeded);
    assert!(answers[1].succeeded, "{}", answers[1].text);
    assert_eq!(connector.tools(), ["odoo_read", "odoo_write", "odoo_read"]);
    assert_eq!(
        journal(&world.pool, world.run_id).await,
        [
            row("odoo_write", "write", "denied", Some("write_not_granted")),
            row("odoo_write", "write", "succeeded", None),
        ]
    );
    assert_eq!(
        world
            .usecase
            .count_integration_writes(world.organization_id, world.run_id)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn an_invalid_call_is_answered_without_a_journal_row_or_a_request() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world.script(
        &seen,
        vec![
            world.call("odoo_write", Slot::Staging, json!({"model": "res.partner"})),
            Move::Call("drop_database", json!({})),
            Move::Call("odoo_count", json!("not an object")),
        ],
    );

    world.run(script, &connector).await;

    let answers = seen.answers();
    assert_eq!(answers.len(), 3);
    for answer in &answers {
        assert!(!answer.succeeded);
        assert!(answer.text.starts_with("Invalid call: "), "{}", answer.text);
    }
    assert_eq!(answers[0].summary, "odoo_write: invalid call");
    assert_eq!(answers[1].summary, "drop_database: invalid call");
    assert_eq!(answers[2].summary, "odoo_count: invalid call");
    assert!(journal(&world.pool, world.run_id).await.is_empty());
    assert!(connector.tools().is_empty());
    let step = step(&world.pool, world.run_id).await;
    assert_eq!(step.output["tool_calls"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn the_intervention_reports_on_the_card_and_leaves_it_in_progress() {
    let Some(world) = world(&[(Slot::Prod, Access::Read), (Slot::Staging, Access::Write)]).await
    else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let mut script = world.script(
        &seen,
        vec![
            Move::Call("list_instances", json!({})),
            world.call("odoo_count", Slot::Prod, json!({"model": "res.partner"})),
            world.call(
                "odoo_write",
                Slot::Prod,
                json!({"model": "res.partner", "ids": [7], "values": {"name": "X"}}),
            ),
            world.call(
                "odoo_write",
                Slot::Staging,
                json!({"model": "res.partner", "ids": [7], "values": {"name": "Acme SA"}}),
            ),
        ],
    );
    script.text = "J'ai corrigé le partenaire, prévenir @octocat.".into();

    world.run(script, &connector).await;

    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("succeeded".into(), None)
    );
    assert_eq!(column(&world.pool, world.card.id).await, "in_progress");
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].0, "user-1");
    assert_eq!(posted[0].1.as_deref(), Some(INTEGRATOR_AGENT));
    let body = &posted[0].2;
    assert!(body.starts_with("**Intervention terminée**"), "{body}");
    assert!(body.contains("J'ai corrigé le partenaire, prévenir @\u{200B}octocat."));
    assert!(!body.contains("@octocat"));
    assert!(body.contains("1 lecture, 1 écriture, 1 refus"), "{body}");
    assert!(body.contains(&format!(
        "https://console.test/kanban?card={}",
        world.card.id.0
    )));
    assert!(body.contains("$0.001234"));
    assert!(!body.contains("Arrêt"));
    assert!(!body.contains(KEY));

    let step = step(&world.pool, world.run_id).await;
    assert_eq!(
        (step.status.as_str(), step.kind.as_str()),
        ("succeeded", "intervene")
    );
    assert_eq!(step.attempt, 1);
    assert_eq!(step.error, None);
    assert_eq!(step.cost_micros, 1_234);
    assert_eq!((step.input_tokens, step.output_tokens), (100, 20));
    assert!(step.latency_ms.is_some());
    let calls = step.output["tool_calls"].as_array().unwrap();
    let names: Vec<&str> = calls
        .iter()
        .map(|call| call["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["list_instances", "odoo_count", "odoo_write", "odoo_write"]
    );
    assert_eq!(
        step.output["report"],
        "J'ai corrigé le partenaire, prévenir @octocat."
    );
    assert_eq!(step.output["stopped"], Value::Null);
    assert_eq!(
        step.output["comment_id"].as_str().unwrap().len(),
        36,
        "the step remembers its comment"
    );
    assert!(!step.output.to_string().contains(KEY));
}

#[tokio::test]
async fn the_prompt_frames_the_card_the_instructions_and_the_granted_instances() {
    let Some(world) = world(&[(Slot::Prod, Access::Read), (Slot::Staging, Access::Write)]).await
    else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());

    world.run(world.script(&seen, vec![]), &connector).await;

    let request = seen.request();
    let prompt = &request.prompt;
    assert_eq!(request.target.model.model, "chat");
    assert_eq!(request.limits, intervention_limits());
    assert_eq!(request.deadline, INTERVENTION_DEADLINE);
    assert_eq!(prompt.matches("<card>").count(), 1);
    assert_eq!(prompt.matches("</card>").count(), 1);
    assert!(prompt.contains("Vérifier les factures d'avril"));
    assert!(prompt.contains("Dans l'Odoo du client"));
    assert_eq!(prompt.matches("<instructions>").count(), 1);
    assert!(prompt.contains("Ne touche pas aux factures payées"));
    assert!(prompt.contains(&format!(
        "Odoo staging — odoo_json2 — staging — écriture — {}",
        world.staging.id.0
    )));
    assert!(prompt.contains(&format!(
        "Odoo prod — odoo_json2 — production — lecture — {}",
        world.prod.id.0
    )));
    assert!(!prompt.contains(&world.other.id.0.to_string()));
    assert!(!prompt.contains(KEY));
}

#[tokio::test]
async fn a_run_claimed_again_after_a_recorded_write_fails_and_never_runs_the_agent() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    world.record_write("pending", None).await;
    world.crashed().await;
    let (seen, connector) = (Seen::default(), Connector::default());

    world
        .run(
            world.script(&seen, vec![Move::Call("list_instances", json!({}))]),
            &connector,
        )
        .await;

    assert_eq!(seen.runs(), 0, "the agent was not started again");
    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("failed".into(), Some(INTERRUPTED.into()))
    );
    let step = step(&world.pool, world.run_id).await;
    assert_eq!(step.status, "failed");
    assert_eq!(step.error.as_deref(), Some(INTERRUPTED));
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].1.as_deref(), Some(INTEGRATOR_AGENT));
    assert!(posted[0].2.starts_with("**Intervention échouée**"));
    assert!(posted[0].2.contains(INTERRUPTED));
    assert!(posted[0].2.contains("1 écriture"));
    assert_eq!(column(&world.pool, world.card.id).await, "in_progress");
}

#[tokio::test]
async fn every_write_that_was_not_refused_blocks_a_resume() {
    for outcome in ["succeeded", "failed"] {
        let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
            return;
        };
        world.record_write(outcome, None).await;
        world.crashed().await;
        let (seen, connector) = (Seen::default(), Connector::default());

        world.run(world.script(&seen, vec![]), &connector).await;

        assert_eq!(seen.runs(), 0, "{outcome}");
        assert_eq!(
            run_row(&world.pool, world.run_id).await.1.as_deref(),
            Some(INTERRUPTED),
            "{outcome}"
        );
    }
}

#[tokio::test]
async fn a_resumed_run_without_a_write_runs_again() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    world
        .record_write("denied", Some("write_not_granted"))
        .await;
    world.crashed().await;
    let (seen, connector) = (Seen::default(), Connector::default());

    world
        .run(
            world.script(&seen, vec![Move::Call("list_instances", json!({}))]),
            &connector,
        )
        .await;

    assert_eq!(seen.runs(), 1);
    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("succeeded".into(), None)
    );
    assert_eq!(step(&world.pool, world.run_id).await.status, "succeeded");
    assert_eq!(comments(&world.pool, world.card.id).await.len(), 1);
}

#[tokio::test]
async fn a_run_resumed_after_its_step_was_recorded_only_finishes_and_comments() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let overview = world
        .usecase
        .get_run(human(world.organization_id, false), world.run_id)
        .await
        .unwrap();
    world
        .usecase
        .record_step(
            overview.run,
            Step {
                id: StepId::new(),
                run_id: world.run_id,
                organization_id: world.organization_id,
                seq: 0,
                kind: StepKind::Intervene,
                agent: Some(INTEGRATOR_AGENT.into()),
                status: StepStatus::Succeeded,
                attempt: 1,
                output: Some(json!({"tool_calls": [], "stopped": null, "report": "Fait."})),
                usage: Usage::default(),
                cost_micros: 4_200,
                latency_ms: Some(10),
                error: None,
                started_at: Utc::now(),
                finished_at: Some(Utc::now()),
            },
        )
        .await
        .unwrap();
    world.crashed().await;
    let (seen, connector) = (Seen::default(), Connector::default());

    world.run(world.script(&seen, vec![]), &connector).await;

    assert_eq!(seen.runs(), 0);
    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("succeeded".into(), None)
    );
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert!(posted[0].2.starts_with("**Intervention terminée**"));
    assert!(posted[0].2.contains("Fait."));
    assert!(posted[0].2.contains("$0.004200"));
    assert!(step(&world.pool, world.run_id).await.output["comment_id"].is_string());
}

#[tokio::test]
async fn the_run_heartbeat_moves_after_every_tool_call() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world.script(
        &seen,
        vec![
            Move::AgeHeartbeat,
            Move::ObserveHeartbeat,
            Move::Call("list_instances", json!({})),
            Move::ObserveHeartbeat,
            Move::AgeHeartbeat,
            world.call("odoo_count", Slot::Staging, json!({"model": "res.partner"})),
            Move::ObserveHeartbeat,
            Move::AgeHeartbeat,
            Move::Call("drop_database", json!({})),
            Move::ObserveHeartbeat,
        ],
    );

    world.run(script, &connector).await;

    let heartbeats: Vec<DateTime<Utc>> = seen
        .heartbeats
        .lock()
        .unwrap()
        .iter()
        .map(|at| at.unwrap())
        .collect();
    let now = Utc::now();
    assert!(now - heartbeats[0] > chrono::Duration::hours(1));
    for fresh in &heartbeats[1..] {
        assert!(now - *fresh < chrono::Duration::minutes(1), "{fresh}");
    }
}

#[tokio::test]
async fn a_model_error_fails_the_run_keeps_the_calls_and_still_comments() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world
        .script(
            &seen,
            vec![
                Move::Call("list_instances", json!({})),
                world.call("odoo_count", Slot::Staging, json!({"model": "res.partner"})),
            ],
        )
        .stopped("model error: the provider closed the stream");

    world.run(script, &connector).await;

    let reason = "model error: the provider closed the stream";
    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("failed".into(), Some(reason.into()))
    );
    let step = step(&world.pool, world.run_id).await;
    assert_eq!(step.status, "failed");
    assert_eq!(step.error.as_deref(), Some(reason));
    assert_eq!(step.output["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(step.output["stopped"], reason);
    assert_eq!(step.cost_micros, 1_234);
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert!(
        posted[0]
            .2
            .starts_with("**Intervention échouée** : model error")
    );
    assert!(
        posted[0].2.contains("1 lecture, 0 écriture, 0 refus"),
        "{}",
        posted[0].2
    );
    assert_eq!(column(&world.pool, world.card.id).await, "in_progress");
}

#[tokio::test]
async fn a_stopped_report_succeeds_and_says_why_it_stopped() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world
        .script(&seen, vec![Move::Call("list_instances", json!({}))])
        .stopped("deadline reached");

    world.run(script, &connector).await;

    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("succeeded".into(), None)
    );
    let posted = comments(&world.pool, world.card.id).await;
    assert!(
        posted[0].2.contains("Arrêt : deadline reached"),
        "{}",
        posted[0].2
    );
}

#[tokio::test]
async fn a_runtime_failure_fails_the_run_keeps_the_calls_and_comments() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let script = world
        .script(&seen, vec![Move::Call("list_instances", json!({}))])
        .failing("the model is unreachable");

    world.run(script, &connector).await;

    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("failed".into(), Some("the model is unreachable".into()))
    );
    let step = step(&world.pool, world.run_id).await;
    assert_eq!(step.status, "failed");
    assert_eq!(step.output["tool_calls"].as_array().unwrap().len(), 1);
    let posted = comments(&world.pool, world.card.id).await;
    assert!(
        posted[0]
            .2
            .starts_with("**Intervention échouée** : the model is unreachable")
    );
}

#[tokio::test]
async fn without_a_usable_model_the_step_fails_and_the_agent_never_starts() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    let mut models = ModelAssignments::defaults("chat", "embed");
    models.integrator = Some(ModelRef {
        provider: ProviderKind::Anthropic,
        model: "claude-sonnet-5-5".into(),
    });
    let pipeline = InterventionPipeline::new(
        world.usecase.clone(),
        world.script(&seen, vec![]),
        connector.clone(),
        InterventionConfig {
            default_models: models,
            ..config()
        },
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(seen.runs(), 0);
    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert!(failure.unwrap().starts_with("aucun modèle utilisable : "));
    assert_eq!(step(&world.pool, world.run_id).await.status, "failed");
    assert_eq!(comments(&world.pool, world.card.id).await.len(), 1);
}

#[tokio::test]
async fn a_finished_or_alive_run_is_not_claimed_again() {
    let Some(world) = world(&[(Slot::Staging, Access::Write)]).await else {
        return;
    };
    let (seen, connector) = (Seen::default(), Connector::default());
    world.run(world.script(&seen, vec![]), &connector).await;
    assert_eq!(seen.runs(), 1);

    world.run(world.script(&seen, vec![]), &connector).await;

    assert_eq!(seen.runs(), 1, "a finished run is left alone");
    assert_eq!(comments(&world.pool, world.card.id).await.len(), 1);
}
