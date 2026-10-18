#![allow(clippy::unwrap_used)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use capitainerie_common::{CardId, CoreError, OrganizationId, RunId};
use capitainerie_core::{
    CapitainerieUseCase, ExecuteConfig, ExecutePipeline, Executor,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::{
            Card, Column,
            service::{CreateCard, MoveCard},
        },
        delegation::DelegatedToken,
        execution::{
            DraftPullRequest, PullRequestUrl, TaskGrant, TaskLine, TaskMarker, TaskOutcome,
            execution_branch,
            ports::{MockCodeHost, MockTaskRunner},
        },
        provider::ModelAssignments,
        run::{
            StepKind,
            step::{Pricing, Step, StepStatus},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../libs/core/tests/fixtures/claude-stream.jsonl");
const PR_URL: &str = "https://github.com/acme/web/pull/7";

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

struct World {
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization_id: OrganizationId,
    installation_id: i64,
    run_id: RunId,
    card: Card,
}

fn fresh_installation() -> i64 {
    (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64
}

async fn world() -> Option<World> {
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
        .with_cipher(SecretCipher::new(&[7u8; 32]).unwrap());
    let installation_id = fresh_installation();
    usecase
        .connect_github(
            Subject {
                is_admin: true,
                ..human(organization_id)
            },
            installation_id,
            "acme".into(),
        )
        .await
        .unwrap();
    let proposed = usecase
        .create_card(
            human(organization_id),
            CreateCard {
                title: "Ajouter un CHANGELOG".into(),
                description: "À la racine".into(),
                provenance: None,
                run_id: None,
            },
        )
        .await
        .unwrap();
    let todo = usecase
        .move_card(
            human(organization_id),
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
        .execute_card(
            human(organization_id),
            todo.id,
            "acme/web".parse().unwrap(),
            "main".into(),
            todo.version,
        )
        .await
        .unwrap();
    Some(World {
        pool,
        usecase,
        organization_id,
        installation_id,
        run_id: started.run_id,
        card: started.card,
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
        prices: vec![(
            "claude-sonnet-5-5".into(),
            Pricing {
                input_per_million_micros: 1_000_000,
                output_per_million_micros: 2_000_000,
            },
        )],
    }
}

fn lines_of(sample: &str) -> Vec<TaskLine> {
    sample
        .lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            if value["type"] == "capitainerie" {
                TaskLine::Capitainerie(serde_json::from_value(value).unwrap())
            } else {
                TaskLine::Agent(value)
            }
        })
        .collect()
}

#[derive(Clone, Default)]
struct Seen {
    token: Arc<Mutex<Option<String>>>,
    starts: Arc<Mutex<u32>>,
    pull_requests: Arc<Mutex<Vec<DraftPullRequest>>>,
}

const PUSHED_SHA: &str = "3f9c2a7d1e5b48a6c0d7e9f1a2b3c4d5e6f70812";

fn runner_ending(seen: &Seen, lines: Vec<TaskLine>, outcome: TaskOutcome) -> MockTaskRunner {
    let mut runner = MockTaskRunner::new();
    let (token, starts) = (seen.token.clone(), seen.starts.clone());
    runner.expect_start().returning(move |spec| {
        *token.lock().unwrap() = Some(spec.task_token.expose().to_owned());
        *starts.lock().unwrap() += 1;
        Box::pin(std::future::ready(Ok(())))
    });
    runner.expect_follow().returning(move |_, sender| {
        let lines = lines.clone();
        let outcome = outcome.clone();
        Box::pin(async move {
            for line in lines {
                sender.send(line).await.unwrap();
            }
            Ok(outcome)
        })
    });
    runner
}

fn deleting(mut runner: MockTaskRunner) -> MockTaskRunner {
    runner
        .expect_delete()
        .times(1)
        .returning(|_| Box::pin(std::future::ready(Ok(()))));
    runner
}

fn host(seen: &Seen, installation_id: i64) -> MockCodeHost {
    let mut host = MockCodeHost::new();
    let pull_requests = seen.pull_requests.clone();
    host.expect_open_draft_pr()
        .returning(move |installation, pull_request| {
            assert_eq!(installation, installation_id);
            pull_requests.lock().unwrap().push(pull_request);
            Box::pin(std::future::ready(Ok(PullRequestUrl(PR_URL.into()))))
        });
    host.expect_branch_head()
        .returning(|_, repository, branch| {
            assert_eq!(repository.to_string(), "acme/web");
            assert!(branch.starts_with("capitainerie/"));
            Box::pin(std::future::ready(Ok(Some(PUSHED_SHA.to_owned()))))
        });
    host
}

type Pipeline = ExecutePipeline<MockTaskRunner, MockCodeHost>;

fn pipeline(world: &World, runner: MockTaskRunner, host: MockCodeHost) -> Pipeline {
    ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Ready { runner, host },
        config(),
    )
}

async fn run_row(pool: &PgPool, run_id: RunId) -> (String, Option<String>) {
    sqlx::query_as("SELECT status, failure FROM runs WHERE id = $1")
        .bind(run_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn grants(pool: &PgPool, run_id: RunId) -> (i64, i64) {
    sqlx::query_as("SELECT count(*), count(revoked_at) FROM task_grants WHERE run_id = $1")
        .bind(run_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn delegations(pool: &PgPool, run_id: RunId) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM run_delegations WHERE run_id = $1")
        .bind(run_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn step(pool: &PgPool, run_id: RunId) -> (String, String, i64, i64, serde_json::Value) {
    sqlx::query_as(
        "SELECT status, kind, cost_micros, input_tokens, output FROM run_steps WHERE run_id = $1",
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

async fn assert_closed(world: &World) {
    let (total, revoked) = grants(&world.pool, world.run_id).await;
    assert_eq!(total, revoked, "every task grant is revoked");
    assert_eq!(delegations(&world.pool, world.run_id).await, 0);
}

#[tokio::test]
async fn a_pushed_branch_opens_a_draft_pull_request_and_comments_the_card() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let lines = lines_of(SAMPLE);
    let token_probe = seen.token.clone();
    let mut runner = MockTaskRunner::new();
    let (token, starts) = (seen.token.clone(), seen.starts.clone());
    runner.expect_start().returning(move |spec| {
        *token.lock().unwrap() = Some(spec.task_token.expose().to_owned());
        *starts.lock().unwrap() += 1;
        assert_eq!(spec.branch, execution_branch(spec.run_id));
        assert_eq!(spec.model.as_str(), "claude-sonnet-5-5");
        assert_eq!(spec.max_turns, 50);
        assert!(spec.prompt.contains("Ajouter un CHANGELOG"));
        Box::pin(std::future::ready(Ok(())))
    });
    runner.expect_follow().returning(move |_, sender| {
        let lines = lines.clone();
        let token = token_probe.lock().unwrap().clone().unwrap();
        Box::pin(async move {
            sender
                .send(TaskLine::Agent(serde_json::json!({
                    "type": "assistant",
                    "message": {"content": [{
                        "type": "tool_use", "id": "toolu_leak", "name": "Bash",
                        "input": {"command": format!("echo {token}")}
                    }]}
                })))
                .await
                .unwrap();
            sender
                .send(TaskLine::Agent(serde_json::json!({
                    "type": "user",
                    "message": {"content": [{
                        "type": "tool_result", "tool_use_id": "toolu_leak",
                        "content": format!("{token}\n"), "is_error": false
                    }]}
                })))
                .await
                .unwrap();
            for line in lines {
                sender.send(line).await.unwrap();
            }
            Ok(TaskOutcome::Succeeded)
        })
    });
    let pipeline = pipeline(&world, runner, host(&seen, world.installation_id));

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    assert_eq!(*seen.starts.lock().unwrap(), 1);
    let pull_requests = seen.pull_requests.lock().unwrap().clone();
    assert_eq!(pull_requests.len(), 1);
    let pull_request = &pull_requests[0];
    assert_eq!(pull_request.base, "main");
    assert_eq!(pull_request.head, execution_branch(world.run_id));
    assert_eq!(pull_request.title, "Ajouter un CHANGELOG");
    assert!(pull_request.body.contains(&format!(
        "https://console.test/kanban?card={}",
        world.card.id.0
    )));
    assert!(pull_request.body.contains("22174 en entrée, 283 en sortie"));
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].0, "user-1");
    assert_eq!(posted[0].1.as_deref(), Some("capitainerie-agent-executor"));
    assert!(posted[0].2.contains(PR_URL));
    assert!(posted[0].2.contains("I added CHANGELOG.md"));
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE run_id = $1 AND action = 'card.comment' AND actor = 'capitainerie-agent-executor' AND allowed",
    )
    .bind(world.run_id.0)
    .fetch_one(&world.pool)
    .await
    .unwrap();
    assert_eq!(audited, 1);

    let (status, kind, cost, input_tokens, output) = step(&world.pool, world.run_id).await;
    assert_eq!((status.as_str(), kind.as_str()), ("succeeded", "execute"));
    assert_eq!(input_tokens, 22_174);
    assert_eq!(cost, 9_467, "cache reads at 10 %, cache writes at 125 %");
    assert_eq!(output["pull_request"], PR_URL);
    assert_eq!(output["comment"], "posted");
    assert!(output["comment_id"].is_string());
    assert_eq!(output["job_started"], true);
    let model: String = sqlx::query_scalar("SELECT model FROM task_grants WHERE run_id = $1")
        .bind(world.run_id.0)
        .fetch_one(&world.pool)
        .await
        .unwrap();
    assert_eq!(model, "claude-sonnet-5-5");
    assert_eq!(output["tool_calls"].as_array().unwrap().len(), 4);

    assert_closed(&world).await;
    let token = seen.token.lock().unwrap().clone().unwrap();
    let hashed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM task_grants WHERE run_id = $1 AND token_sha256 = sha256($2::bytea)",
    )
    .bind(world.run_id.0)
    .bind(token.as_bytes())
    .fetch_one(&world.pool)
    .await
    .unwrap();
    assert_eq!(hashed, 1, "only the hash of the token is stored");
    let stored = format!(
        "{output} {} {} {}",
        pull_request.body, posted[0].2, pull_request.title
    );
    assert!(!stored.contains(&token), "the task token reached storage");
    assert!(output.to_string().contains("***"));
}

#[tokio::test]
async fn without_changes_no_pull_request_is_opened_and_the_card_says_so() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let mut lines = lines_of(SAMPLE);
    lines.pop();
    lines.push(TaskLine::Capitainerie(TaskMarker::NoChanges));
    let runner = runner_ending(&seen, lines, TaskOutcome::Succeeded);
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    assert!(seen.pull_requests.lock().unwrap().is_empty());
    let posted = comments(&world.pool, world.card.id).await;
    assert!(posted[0].2.starts_with("Aucun changement"));
    assert!(posted[0].2.contains("I added CHANGELOG.md"));
    let (status, _, _, _, output) = step(&world.pool, world.run_id).await;
    assert_eq!(status, "succeeded");
    assert_eq!(output["no_changes"], true);
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_task_past_its_deadline_fails_the_run_and_revokes_the_token() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let runner = deleting(runner_ending(
        &seen,
        Vec::new(),
        TaskOutcome::DeadlineExceeded,
    ));
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert_eq!(failure.as_deref(), Some("durée maximale dépassée"));
    assert_eq!(step(&world.pool, world.run_id).await.0, "failed");
    assert!(
        comments(&world.pool, world.card.id).await[0]
            .2
            .contains("durée maximale dépassée")
    );
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_failed_task_fails_the_run_with_its_reason() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let runner = deleting(runner_ending(
        &seen,
        Vec::new(),
        TaskOutcome::Failed {
            reason: "OOMKilled".into(),
        },
    ));
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert_eq!(failure.as_deref(), Some("OOMKilled"));
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_runner_that_keeps_failing_is_retried_then_fails_the_run_and_revokes() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let mut runner = MockTaskRunner::new();
    let starts = seen.starts.clone();
    runner.expect_start().times(2).returning(move |_| {
        *starts.lock().unwrap() += 1;
        Box::pin(std::future::ready(Err(CoreError::Internal(
            "the cluster is unreachable".into(),
        ))))
    });
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert_eq!(failure.as_deref(), Some("the cluster is unreachable"));
    assert_eq!(*seen.starts.lock().unwrap(), 2);
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_transient_runner_error_is_retried_and_the_run_succeeds() {
    let Some(world) = world().await else { return };
    let mut lines = lines_of(SAMPLE);
    lines.pop();
    lines.push(TaskLine::Capitainerie(TaskMarker::NoChanges));
    let mut runner = MockTaskRunner::new();
    let calls = Arc::new(Mutex::new(0u32));
    runner.expect_start().times(2).returning(move |_| {
        let mut calls = calls.lock().unwrap();
        *calls += 1;
        let result = if *calls == 1 {
            Err(CoreError::Internal("try again".into()))
        } else {
            Ok(())
        };
        Box::pin(std::future::ready(result))
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
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    assert_closed(&world).await;
}

fn no_changes_lines() -> Vec<TaskLine> {
    let mut lines = lines_of(SAMPLE);
    lines.pop();
    lines.push(TaskLine::Capitainerie(TaskMarker::NoChanges));
    lines
}

fn following(lines: Vec<TaskLine>, runner: &mut MockTaskRunner) {
    runner.expect_follow().times(1).returning(move |_, sender| {
        let lines = lines.clone();
        Box::pin(async move {
            for line in lines {
                sender.send(line).await.unwrap();
            }
            Ok(TaskOutcome::Succeeded)
        })
    });
}

async fn crashed_attempt(world: &World, token: &str, output: Option<serde_json::Value>) {
    let previous = world
        .usecase
        .claim_execution(world.run_id, Utc::now() - chrono::Duration::hours(1), None)
        .await
        .unwrap()
        .unwrap();
    assert!(!previous.resumed);
    let expires_at = Utc::now() + chrono::Duration::minutes(30);
    world
        .usecase
        .issue_task_token(
            world.run_id,
            DelegatedToken {
                token: SecretToken::new(token),
                expires_at,
            },
            TaskGrant {
                run_id: world.run_id,
                organization_id: world.organization_id,
                repository: "acme/web".parse().unwrap(),
                branch: execution_branch(world.run_id),
                model: "claude-sonnet-5-5".into(),
                expires_at,
                revoked_at: None,
            },
        )
        .await
        .unwrap();
    if let Some(output) = output {
        let step = Step {
            id: capitainerie_common::StepId::new(),
            run_id: world.run_id,
            organization_id: world.organization_id,
            seq: 0,
            kind: StepKind::Execute,
            agent: Some("capitainerie-agent-executor".into()),
            status: StepStatus::Running,
            attempt: 1,
            output: Some(output),
            usage: Default::default(),
            cost_micros: 0,
            latency_ms: None,
            error: None,
            started_at: Utc::now(),
            finished_at: None,
        };
        world.usecase.record_step(previous.run, step).await.unwrap();
    }
    stale_heartbeat(world).await;
}

async fn stale_heartbeat(world: &World) {
    sqlx::query("UPDATE runs SET heartbeat_at = now() - interval '2 hours' WHERE id = $1")
        .bind(world.run_id.0)
        .execute(&world.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_resumed_run_attaches_to_its_job_with_the_same_token() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let first_token = format!("first-attempt-{}", world.run_id);
    crashed_attempt(&world, &first_token, None).await;
    let mut runner = MockTaskRunner::new();
    runner
        .expect_attach()
        .times(1)
        .returning(|_| Box::pin(std::future::ready(Ok(true))));
    following(no_changes_lines(), &mut runner);
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(*seen.starts.lock().unwrap(), 0, "no second Job");
    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    let (total, revoked) = grants(&world.pool, world.run_id).await;
    assert_eq!((total, revoked), (1, 1), "no second grant was issued");
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_resume_without_a_job_fails_and_starts_nothing() {
    let Some(world) = world().await else { return };
    crashed_attempt(&world, &format!("first-{}", world.run_id), None).await;
    let mut runner = MockTaskRunner::new();
    runner
        .expect_attach()
        .times(1)
        .returning(|_| Box::pin(std::future::ready(Ok(false))));
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert_eq!(failure.as_deref(), Some("tâche introuvable à la reprise"));
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_run_that_crashed_before_its_job_started_may_start_it() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    crashed_attempt(
        &world,
        &format!("first-{}", world.run_id),
        Some(serde_json::json!({"job_started": false})),
    )
    .await;
    let mut runner = runner_ending(&seen, no_changes_lines(), TaskOutcome::Succeeded);
    runner
        .expect_attach()
        .times(1)
        .returning(|_| Box::pin(std::future::ready(Ok(false))));
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(*seen.starts.lock().unwrap(), 1);
    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_pull_request_opened_before_a_crash_is_not_opened_again_after_two_resumes() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let first_token = format!("first-{}", world.run_id);
    crashed_attempt(
        &world,
        &first_token,
        Some(serde_json::json!({
            "job_started": true,
            "branch": execution_branch(world.run_id),
            "sha": PUSHED_SHA,
            "pull_request": PR_URL,
        })),
    )
    .await;

    let mut hanging = MockTaskRunner::new();
    hanging
        .expect_attach()
        .returning(|_| Box::pin(std::future::ready(Ok(true))));
    hanging
        .expect_follow()
        .returning(|_, _| Box::pin(std::future::pending()));
    let interrupted = pipeline(&world, hanging, MockCodeHost::new());
    let crash = tokio::time::timeout(
        Duration::from_millis(600),
        interrupted.execute(world.run_id, None),
    )
    .await;
    assert!(crash.is_err(), "the first resume never ended");
    let carried: serde_json::Value =
        sqlx::query_scalar("SELECT output FROM run_steps WHERE run_id = $1")
            .bind(world.run_id.0)
            .fetch_one(&world.pool)
            .await
            .unwrap();
    assert_eq!(carried["pull_request"], PR_URL);
    assert_eq!(carried["sha"], PUSHED_SHA);
    stale_heartbeat(&world).await;

    let mut runner = MockTaskRunner::new();
    runner
        .expect_attach()
        .times(1)
        .returning(|_| Box::pin(std::future::ready(Ok(true))));
    following(lines_of(SAMPLE), &mut runner);
    let pipeline = pipeline(&world, runner, MockCodeHost::new());
    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "succeeded");
    assert!(seen.pull_requests.lock().unwrap().is_empty());
    let (_, _, _, _, output) = step(&world.pool, world.run_id).await;
    assert_eq!(output["pull_request"], PR_URL);
    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert!(posted[0].2.contains(PR_URL));
}

#[tokio::test]
async fn a_comment_posted_before_a_crash_is_not_posted_again() {
    let Some(world) = world().await else { return };
    crashed_attempt(
        &world,
        &format!("first-{}", world.run_id),
        Some(serde_json::json!({"job_started": true})),
    )
    .await;
    let already = world
        .usecase
        .comment_card_as_executor(world.run_id, world.card.id, "déjà dit".into())
        .await
        .unwrap();
    let mut runner = MockTaskRunner::new();
    runner
        .expect_attach()
        .returning(|_| Box::pin(std::future::ready(Ok(true))));
    following(no_changes_lines(), &mut runner);
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let posted = comments(&world.pool, world.card.id).await;
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].2, "déjà dit");
    let (_, _, _, _, output) = step(&world.pool, world.run_id).await;
    assert_eq!(output["comment_id"], already.0.to_string());
}

#[tokio::test]
async fn a_run_alive_on_another_worker_is_left_alone() {
    let Some(world) = world().await else { return };
    world
        .usecase
        .claim_execution(world.run_id, Utc::now() - chrono::Duration::hours(1), None)
        .await
        .unwrap()
        .unwrap();
    let pipeline = pipeline(&world, MockTaskRunner::new(), MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    assert_eq!(run_row(&world.pool, world.run_id).await.0, "running");
}

#[tokio::test]
async fn text_the_agent_wrote_cannot_mention_anyone() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let mut lines = lines_of(SAMPLE);
    for line in &mut lines {
        if let TaskLine::Agent(value) = line
            && value["type"] == "result"
        {
            value["result"] = serde_json::json!("Merci @octocat, cc @acme/security");
        }
    }
    let runner = runner_ending(&seen, lines, TaskOutcome::Succeeded);
    let pipeline = pipeline(&world, runner, host(&seen, world.installation_id));

    pipeline.execute(world.run_id, None).await.unwrap();

    let pull_requests = seen.pull_requests.lock().unwrap().clone();
    let posted = comments(&world.pool, world.card.id).await;
    for text in [&pull_requests[0].body, &posted[0].2] {
        assert!(text.contains("@\u{200B}octocat"), "{text}");
        assert!(
            !text.contains("@octocat") && !text.contains("@acme"),
            "{text}"
        );
    }
}

#[tokio::test]
async fn a_branch_that_is_not_where_the_task_says_fails_the_run_before_any_pull_request() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let runner = deleting(runner_ending(
        &seen,
        lines_of(SAMPLE),
        TaskOutcome::Succeeded,
    ));
    let mut host = MockCodeHost::new();
    host.expect_branch_head().returning(|_, _, _| {
        Box::pin(std::future::ready(Ok(Some(
            "0000000000000000000000000000000000000000".to_owned(),
        ))))
    });
    let pipeline = pipeline(&world, runner, host);

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert_eq!(failure.as_deref(), Some("branche absente ou différente"));
    assert!(seen.pull_requests.lock().unwrap().is_empty());
    assert_closed(&world).await;
}

#[tokio::test]
async fn a_missing_branch_fails_the_run_too() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let runner = deleting(runner_ending(
        &seen,
        lines_of(SAMPLE),
        TaskOutcome::Succeeded,
    ));
    let mut host = MockCodeHost::new();
    host.expect_branch_head()
        .returning(|_, _, _| Box::pin(std::future::ready(Ok(None))));
    let pipeline = pipeline(&world, runner, host);

    pipeline.execute(world.run_id, None).await.unwrap();

    let (_, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(failure.as_deref(), Some("branche absente ou différente"));
}

#[tokio::test]
async fn an_agent_that_ran_out_of_turns_without_pushing_fails_the_run() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let mut lines = no_changes_lines();
    for line in &mut lines {
        if let TaskLine::Agent(value) = line
            && value["type"] == "result"
        {
            value["subtype"] = serde_json::json!("error_max_turns");
        }
    }
    let runner = deleting(runner_ending(&seen, lines, TaskOutcome::Succeeded));
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert_eq!(
        failure.as_deref(),
        Some("arrêté : nombre maximal de tours atteint")
    );
    let posted = comments(&world.pool, world.card.id).await;
    assert!(
        posted[0]
            .2
            .contains("arrêté : nombre maximal de tours atteint")
    );
    assert!(!posted[0].2.starts_with("Aucun changement"));
}

#[tokio::test]
async fn a_model_without_a_known_price_costs_what_claude_code_reported() {
    let Some(world) = world().await else { return };
    let seen = Seen::default();
    let runner = runner_ending(&seen, no_changes_lines(), TaskOutcome::Succeeded);
    let mut unpriced = config();
    unpriced.prices.clear();
    let pipeline = ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Ready {
            runner,
            host: MockCodeHost::new(),
        },
        unpriced,
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    let (_, _, cost, _, _) = step(&world.pool, world.run_id).await;
    assert_eq!(cost, 29_100);
}

#[tokio::test]
async fn a_disabled_executor_fails_the_run_at_once_with_a_clear_reason() {
    let Some(world) = world().await else { return };
    let pipeline: Pipeline = ExecutePipeline::new(
        world.usecase.clone(),
        Executor::Disabled {
            reason: "les exécutions sont désactivées (EXECUTOR_NAMESPACE)".into(),
        },
        config(),
    );

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert_eq!(
        failure.as_deref(),
        Some("les exécutions sont désactivées (EXECUTOR_NAMESPACE)")
    );
    assert_eq!(delegations(&world.pool, world.run_id).await, 0);
}

#[tokio::test]
async fn a_disconnected_github_fails_the_run_before_any_job_starts() {
    let Some(world) = world().await else { return };
    world
        .usecase
        .disconnect_github(Subject {
            is_admin: true,
            ..human(world.organization_id)
        })
        .await
        .unwrap();
    let pipeline = pipeline(&world, MockTaskRunner::new(), MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, failure) = run_row(&world.pool, world.run_id).await;
    assert_eq!(status, "failed");
    assert!(failure.unwrap().contains("GitHub"));
    assert_closed(&world).await;
}

#[tokio::test]
async fn the_timeline_is_saved_while_the_task_runs() {
    let Some(world) = world().await else { return };
    let lines = no_changes_lines();
    let live = Arc::new(Mutex::new(None));
    let mut runner = MockTaskRunner::new();
    runner
        .expect_start()
        .returning(|_| Box::pin(std::future::ready(Ok(()))));
    let (pool, run_id, observed) = (world.pool.clone(), world.run_id, live.clone());
    runner.expect_follow().returning(move |_, sender| {
        let lines = lines.clone();
        let (pool, observed) = (pool.clone(), observed.clone());
        Box::pin(async move {
            for line in lines {
                sender.send(line).await.unwrap();
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
            let output: Option<serde_json::Value> =
                sqlx::query_scalar("SELECT output FROM run_steps WHERE run_id = $1")
                    .bind(run_id.0)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            let status: String =
                sqlx::query_scalar("SELECT status FROM run_steps WHERE run_id = $1")
                    .bind(run_id.0)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            *observed.lock().unwrap() = Some((status, output));
            Ok(TaskOutcome::Succeeded)
        })
    });
    let pipeline = pipeline(&world, runner, MockCodeHost::new());

    pipeline.execute(world.run_id, None).await.unwrap();

    let (status, output) = live.lock().unwrap().clone().unwrap();
    assert_eq!(status, "running");
    assert_eq!(output.unwrap()["tool_calls"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn the_meeting_pipeline_leaves_card_runs_to_the_executor() {
    let Some(world) = world().await else { return };

    assert!(world.usecase.is_execution_run(world.run_id).await.unwrap());
    let claimed = world
        .usecase
        .claim_run(world.run_id, Utc::now(), None)
        .await
        .unwrap();

    assert!(claimed.is_none());
    assert_eq!(run_row(&world.pool, world.run_id).await.0, "queued");
    assert!(
        !world
            .usecase
            .is_execution_run(RunId(Uuid::now_v7()))
            .await
            .unwrap()
    );
}
