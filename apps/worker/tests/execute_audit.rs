#![allow(clippy::unwrap_used)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use aws_lc_rs::digest;
use base64::{Engine, engine::general_purpose::STANDARD};
use capitainerie_common::{CardId, OrganizationId, RunId};
use capitainerie_core::{
    CapitainerieUseCase, ExecuteConfig, ExecutePipeline, Executor,
    domain::{
        agent::SecretToken,
        agent_task::{RepositoryTarget, StartAgentRun},
        authorization::Subject,
        card::{Card, edit::EditCard, service::CreateCard},
        client::links::Change,
        crew::service::CreateAgent,
        execution::{
            ExecutionMode, ExecutorStatus, GithubAppCredentials, TaskLine, TaskMarker, TaskOutcome,
            TaskSpec,
            ports::{MockCodeHost, MockTaskRunner},
        },
        provider::{ModelAssignments, ProviderKind},
        skill::service::CreateSkill,
        user_provider::service::PutUserProvider,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../libs/core/tests/fixtures/claude-stream.jsonl");
const REPORT: &str = "# Audit de acme/web\n\n## Constats\n\n- Une dépendance obsolète.\n";

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

fn fresh_id() -> i64 {
    (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64
}

struct World {
    pool: PgPool,
    usecase: CapitainerieUseCase,
    run_id: RunId,
    card: Card,
    agent_id: Uuid,
}

/// A card assigned to an auditing agent, whose read-only run on acme/web is
/// queued the way the API queues it.
async fn world(allow_writes: bool) -> Option<World> {
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
    usecase
        .save_github_app(
            human(organization_id, true),
            GithubAppCredentials {
                organization_id,
                app_id: fresh_id(),
                slug: "capitainerie-test".into(),
                client_id: "Iv1.test".into(),
                client_secret: SecretToken::new("oauth-secret"),
                private_key_pem: SecretToken::new("pem"),
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
    usecase
        .connect_github(human(organization_id, true), fresh_id(), "acme".into())
        .await
        .unwrap();
    usecase
        .put_user_provider(PutUserProvider {
            subject: human(organization_id, false),
            kind: ProviderKind::Anthropic,
            secret: Some(SecretToken::new("sk-ant-api03-abcdefgh1234")),
            base_url: None,
            default_model: "claude-haiku-5-5".into(),
            is_default: None,
        })
        .await
        .unwrap();
    usecase
        .record_executor_status(&ExecutorStatus {
            enabled: true,
            reason: None,
        })
        .await
        .unwrap();
    usecase
        .create_skill(
            human(organization_id, false),
            CreateSkill {
                slug: "owasp".into(),
                name: "OWASP".into(),
                description: String::new(),
                content: "Cherche les injections SQL.".into(),
                source: "manual".into(),
            },
        )
        .await
        .unwrap();
    let agent = usecase
        .create_crew_agent(
            human(organization_id, false),
            CreateAgent {
                slug: "auditrice".into(),
                name: "Auditrice".into(),
                role: String::new(),
                description: String::new(),
                model: "claude-opus-5".into(),
                hue: 0,
                reports_to: None,
                instructions: "Tu es une auditrice sécurité exigeante.".into(),
                connector_grants: vec![],
                repository_grants: vec!["acme/web".into()],
            },
        )
        .await
        .unwrap();
    usecase
        .update_crew_agent(
            human(organization_id, false),
            agent.id,
            capitainerie_core::domain::crew::service::UpdateAgent {
                skills: Some(vec!["owasp".into()]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let card = usecase
        .create_card_linked(
            human(organization_id, false),
            CreateCard {
                title: "Audite le dépôt acme/web".into(),
                description: "Selon ces instructions : cherche les failles.".into(),
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
    let card = usecase
        .edit_card(human(organization_id, false), card.id, edit)
        .await
        .unwrap();
    let started = usecase
        .start_agent_repository_run(
            human(organization_id, false),
            card.id,
            StartAgentRun {
                expected_version: card.version,
                allow_writes,
            },
            RepositoryTarget::new("acme/web".parse().unwrap(), Some("main".into())).unwrap(),
        )
        .await
        .unwrap();
    Some(World {
        pool,
        usecase,
        run_id: started.run_id,
        card,
        agent_id: agent.id.0,
    })
}

fn config() -> ExecuteConfig {
    ExecuteConfig {
        max_attempts: 2,
        retry_base: Duration::from_millis(1),
        stale_after: Duration::from_secs(600),
        max_turns: 50,
        deadline: Duration::from_secs(1800),
        progress_every: Duration::from_millis(25),
        console_url: Some("https://console.test".into()),
        default_models: ModelAssignments::defaults("chat", "embed"),
        prices: vec![],
    }
}

/// The recorded stream without its push, then what an audit's entrypoint
/// prints: the report in chunks, its digest, the end marker.
fn audit_lines(summary: bool, report: Option<&str>) -> Vec<TaskLine> {
    let mut lines: Vec<TaskLine> = SAMPLE
        .lines()
        .filter_map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            if value["type"] == "capitainerie" {
                return None;
            }
            if value["type"] == "result" && !summary {
                return None;
            }
            Some(TaskLine::Agent(value))
        })
        .collect();
    if let Some(report) = report {
        let encoded = STANDARD.encode(report);
        for chunk in encoded.as_bytes().chunks(16) {
            lines.push(TaskLine::Capitainerie(TaskMarker::ReportChunk {
                data: String::from_utf8(chunk.to_vec()).unwrap(),
            }));
        }
        let hash = digest::digest(&digest::SHA256, report.as_bytes());
        lines.push(TaskLine::Capitainerie(TaskMarker::ReportEnd {
            sha256: hash
                .as_ref()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            bytes: report.len() as u64,
            truncated: false,
        }));
    }
    lines.push(TaskLine::Capitainerie(TaskMarker::AuditDone));
    lines
}

fn runner(lines: Vec<TaskLine>, specs: Arc<Mutex<Vec<TaskSpec>>>) -> MockTaskRunner {
    let mut runner = MockTaskRunner::new();
    runner.expect_start().returning(move |spec| {
        specs.lock().unwrap().push(spec.clone());
        Box::pin(std::future::ready(Ok(())))
    });
    runner.expect_follow().returning(move |_, sender| {
        let lines = lines.clone();
        Box::pin(async move {
            for line in lines {
                sender.send(line).await.unwrap();
            }
            Ok(TaskOutcome::Succeeded)
        })
    });
    runner
        .expect_delete()
        .returning(|_| Box::pin(std::future::ready(Ok(()))));
    runner
}

/// No expectation: any call to GitHub (branch check, pull request) panics.
fn untouched_host() -> MockCodeHost {
    MockCodeHost::new()
}

async fn run_row(pool: &PgPool, run_id: RunId) -> (String, Option<String>) {
    sqlx::query_as("SELECT status, failure FROM runs WHERE id = $1")
        .bind(run_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn comments(pool: &PgPool, card_id: CardId) -> Vec<(Option<String>, String)> {
    sqlx::query_as(
        "SELECT author_actor, body FROM card_comments WHERE card_id = $1 ORDER BY created_at",
    )
    .bind(card_id.0)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn step_output(pool: &PgPool, run_id: RunId) -> (String, String, serde_json::Value) {
    sqlx::query_as("SELECT status, model, output FROM run_steps WHERE run_id = $1")
        .bind(run_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn an_audit_files_its_report_and_comments_as_the_agent_without_any_pull_request() {
    let Some(world) = world(false).await else {
        return;
    };
    let specs = Arc::new(Mutex::new(Vec::new()));
    let pipeline = ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Ready {
            runner: runner(audit_lines(true, Some(REPORT)), Arc::clone(&specs)),
            host: untouched_host(),
        },
        config(),
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(
        run_row(&world.pool, world.run_id).await,
        ("succeeded".into(), None)
    );
    let spec = specs.lock().unwrap()[0].clone();
    assert_eq!(spec.mode, ExecutionMode::Audit);
    assert_eq!(
        spec.model.as_str(),
        "claude-opus-5",
        "the agent's own model"
    );
    assert!(
        spec.prompt
            .starts_with("Tu es une auditrice sécurité exigeante.")
    );
    assert!(spec.prompt.contains("Cherche les injections SQL."));
    assert!(spec.prompt.contains("read-only analysis"));
    assert!(spec.prompt.contains("/workspace/.capitainerie/report.md"));
    assert!(spec.prompt.contains("cherche les failles"));

    let (mode, initiated_by, model): (String, Option<String>, String) =
        sqlx::query_as("SELECT mode, initiated_by, model FROM task_grants WHERE run_id = $1")
            .bind(world.run_id.0)
            .fetch_one(&world.pool)
            .await
            .unwrap();
    assert_eq!(mode, "audit");
    assert_eq!(initiated_by.as_deref(), Some("user-1"));
    assert_eq!(model, "claude-opus-5");
    let frozen: Option<String> =
        sqlx::query_scalar("SELECT model FROM agent_tasks WHERE run_id = $1")
            .bind(world.run_id.0)
            .fetch_one(&world.pool)
            .await
            .unwrap();
    assert_eq!(frozen.as_deref(), Some("claude-opus-5"));

    let (kind, title, content, agent_id, card_id): (String, String, String, Uuid, Uuid) =
        sqlx::query_as(
            "SELECT kind, title, content, agent_id, card_id FROM artifacts WHERE run_id = $1",
        )
        .bind(world.run_id.0)
        .fetch_one(&world.pool)
        .await
        .unwrap();
    assert_eq!(kind, "markdown");
    assert_eq!(title, "Audite le dépôt acme/web");
    assert_eq!(content, REPORT.trim());
    assert_eq!(agent_id, world.agent_id);
    assert_eq!(card_id, world.card.id.0);

    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].0.as_deref(), Some("agent:auditrice"));
    assert!(
        posted[0].1.starts_with("I added CHANGELOG.md"),
        "{}",
        posted[0].1
    );
    assert!(posted[0].1.contains(
        "Rapport complet : artifact « Audite le dépôt acme/web » — https://console.test/artifacts"
    ));

    let (status, model, output) = step_output(&world.pool, world.run_id).await;
    assert_eq!(status, "succeeded");
    assert_eq!(model, "claude-opus-5");
    assert_eq!(output["mode"], "audit");
    assert_eq!(output["report"], "filed");
    assert_eq!(output["artifact"], "published");
    assert!(output["branch"].is_null());
    assert!(output["pull_request"].is_null());
    let (total, revoked): (i64, i64) =
        sqlx::query_as("SELECT count(*), count(revoked_at) FROM task_grants WHERE run_id = $1")
            .bind(world.run_id.0)
            .fetch_one(&world.pool)
            .await
            .unwrap();
    assert_eq!((total, revoked), (1, 1));
}

#[tokio::test]
async fn an_audit_without_report_but_with_a_summary_succeeds_and_says_the_report_is_missing() {
    let Some(world) = world(false).await else {
        return;
    };
    let pipeline = ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Ready {
            runner: runner(audit_lines(true, None), Arc::default()),
            host: untouched_host(),
        },
        config(),
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    let artifacts: i64 = sqlx::query_scalar("SELECT count(*) FROM artifacts WHERE run_id = $1")
        .bind(world.run_id.0)
        .fetch_one(&world.pool)
        .await
        .unwrap();
    assert_eq!(artifacts, 0);
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted[0].0.as_deref(), Some("agent:auditrice"));
    assert!(
        posted[0]
            .1
            .contains("Rapport complet : absent (aucun rapport produit)")
    );
}

#[tokio::test]
async fn an_audit_with_neither_report_nor_summary_fails_as_no_report() {
    let Some(world) = world(false).await else {
        return;
    };
    let pipeline = ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Ready {
            runner: runner(audit_lines(false, None), Arc::default()),
            host: untouched_host(),
        },
        config(),
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert!(failure.unwrap().starts_with("no_report"));
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted[0].0.as_deref(), Some("agent:auditrice"));
    assert!(posted[0].1.contains("no_report"));
}

#[tokio::test]
async fn an_audit_that_never_says_it_ended_fails() {
    let Some(world) = world(false).await else {
        return;
    };
    let mut lines = audit_lines(true, Some(REPORT));
    lines.pop();
    let pipeline = ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Ready {
            runner: runner(lines, Arc::default()),
            host: untouched_host(),
        },
        config(),
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "failed");
}

#[tokio::test]
async fn a_change_run_of_an_agent_keeps_the_change_prompt_after_the_agent() {
    let Some(world) = world(true).await else {
        return;
    };
    let specs = Arc::new(Mutex::new(Vec::new()));
    let mut lines = audit_lines(true, None);
    lines.pop();
    lines.push(TaskLine::Capitainerie(TaskMarker::NoChanges));
    let pipeline = ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Ready {
            runner: runner(lines, Arc::clone(&specs)),
            host: untouched_host(),
        },
        config(),
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    let spec = specs.lock().unwrap()[0].clone();
    assert_eq!(spec.mode, ExecutionMode::Change);
    assert!(
        spec.prompt
            .starts_with("Tu es une auditrice sécurité exigeante.")
    );
    assert!(
        spec.prompt
            .contains("Do not commit or push: it is done for you")
    );
    let mode: String = sqlx::query_scalar("SELECT mode FROM task_grants WHERE run_id = $1")
        .bind(world.run_id.0)
        .fetch_one(&world.pool)
        .await
        .unwrap();
    assert_eq!(mode, "change");
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted[0].0.as_deref(), Some("agent:auditrice"));
    assert!(posted[0].1.starts_with("Aucun changement"));
}
