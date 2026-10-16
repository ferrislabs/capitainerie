#![allow(clippy::unwrap_used)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    AgentDelegation, CapitainerieUseCase, MeetingPipeline, PipelineConfig,
    domain::{
        agent::{
            ExtractedAction, Extraction, SecretToken, ToolCall, Usage, WriteReport, WriteRequest,
            ports::AgentRuntime,
        },
        authorization::Subject,
        delegation::{DelegatedToken, ports::TokenExchanger},
        run::{service::SubmitTranscript, step::Pricing},
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

const BODY: &str = "Claire : il faut migrer la CI vers les runners ARM avant fin octobre.\nMalik : je m’en charge.";
const WRITER: &str = "capitainerie-agent-writer";

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

#[derive(Clone, Default)]
struct FakeRuntime {
    extractions: Arc<AtomicUsize>,
    writes: Arc<std::sync::Mutex<Vec<WriteRequest>>>,
}

impl AgentRuntime for FakeRuntime {
    async fn extract(&self, _transcript: &str) -> Result<Extraction, CoreError> {
        self.extractions.fetch_add(1, Ordering::SeqCst);
        Ok(Extraction {
            actions: vec![
                ExtractedAction {
                    title: "Migrer la CI vers ARM".into(),
                    description: "Avant fin octobre".into(),
                    quote: "Il faut migrer la CI vers les  runners ARM".into(),
                    owner: Some("Malik".into()),
                    due: Some("fin octobre".into()),
                },
                ExtractedAction {
                    title: "Inventé".into(),
                    description: String::new(),
                    quote: "personne n'a dit ça".into(),
                    owner: None,
                    due: None,
                },
            ],
            usage: Usage {
                model: "fake".into(),
                input_tokens: 1_000,
                output_tokens: 200,
            },
        })
    }

    async fn write(&self, request: WriteRequest) -> Result<WriteReport, CoreError> {
        let calls = request
            .actions
            .iter()
            .map(|action| ToolCall {
                tool: "create_card".into(),
                succeeded: true,
                latency_ms: 5,
                summary: action.action.title.clone(),
            })
            .collect();
        self.writes.lock().unwrap().push(request);
        Ok(WriteReport {
            tool_calls: calls,
            usage: Usage {
                model: "fake".into(),
                input_tokens: 500,
                output_tokens: 100,
            },
        })
    }
}

#[derive(Clone, Copy)]
struct FakeExchanger;

impl TokenExchanger for FakeExchanger {
    async fn for_agent(
        &self,
        _issuer: &str,
        user: &SecretToken,
        agent: &str,
    ) -> Result<DelegatedToken, CoreError> {
        Ok(DelegatedToken {
            token: SecretToken::new(format!("{}>{agent}", user.expose())),
            expires_at: Utc::now() + chrono::Duration::minutes(30),
        })
    }

    async fn for_mcp(
        &self,
        _issuer: &str,
        agent_token: &SecretToken,
        _agent: &str,
    ) -> Result<DelegatedToken, CoreError> {
        Ok(DelegatedToken {
            token: SecretToken::new(format!("{}>mcp", agent_token.expose())),
            expires_at: Utc::now() + chrono::Duration::minutes(30),
        })
    }
}

async fn setup() -> Option<(PgPool, CapitainerieUseCase, OrganizationId)> {
    let pool = postgres::connect(&database_url()?, 3).await.unwrap();
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
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::new(&[9u8; 32]).unwrap());
    Some((pool, usecase, OrganizationId(id)))
}

fn config() -> PipelineConfig {
    PipelineConfig {
        writer_agent: WRITER.into(),
        pricing: Pricing {
            input_per_million_micros: 1_000_000,
            output_per_million_micros: 2_000_000,
        },
        max_attempts: 1,
        stale_after: Duration::from_secs(600),
    }
}

async fn submit(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
) -> capitainerie_core::SubmittedTranscript {
    usecase
        .submit_transcript_delegated(
            SubmitTranscript {
                subject: Subject {
                    id: "user-1".into(),
                    organization_id,
                    actor: None,
                },
                title: "Rétro".into(),
                body: BODY.into(),
            },
            AgentDelegation {
                exchanger: &FakeExchanger,
                issuer: "http://idp.test",
                user_token: &SecretToken::new("user"),
                agents: &[WRITER.to_owned()],
            },
        )
        .await
        .unwrap()
}

async fn steps(pool: &PgPool, run_id: Uuid) -> Vec<(String, String, i64)> {
    sqlx::query_as("SELECT kind, status, cost_micros FROM run_steps WHERE run_id = $1 ORDER BY seq")
        .bind(run_id)
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn status(pool: &PgPool, run_id: Uuid) -> String {
    sqlx::query_scalar("SELECT status FROM runs WHERE id = $1")
        .bind(run_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn delegations(pool: &PgPool, run_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM run_delegations WHERE run_id = $1")
        .bind(run_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_run_goes_through_every_step_and_forgets_its_delegation() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT ciphertext FROM run_delegations WHERE run_id = $1")
            .bind(submitted.run_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        !String::from_utf8_lossy(&stored).contains("user>"),
        "the delegation is stored in clear"
    );

    let runtime = FakeRuntime::default();
    let pipeline = MeetingPipeline::new(usecase.clone(), runtime.clone(), FakeExchanger, config());
    pipeline.execute(submitted.run_id, None).await.unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    assert_eq!(
        steps(&pool, submitted.run_id.0).await,
        [
            ("extract".to_owned(), "succeeded".to_owned(), 1_400),
            ("deduplicate".to_owned(), "skipped".to_owned(), 0),
            ("write".to_owned(), "succeeded".to_owned(), 700),
        ]
    );
    let writes = runtime.writes.lock().unwrap().clone();
    assert_eq!(writes.len(), 1);
    assert_eq!(
        writes[0].mcp_token.expose(),
        "user>capitainerie-agent-writer>mcp"
    );
    assert_eq!(writes[0].actions.len(), 1, "the invented quote is dropped");
    let located = &writes[0].actions[0];
    let cited: String = BODY
        .chars()
        .skip(located.start as usize)
        .take((located.end - located.start) as usize)
        .collect();
    assert_eq!(cited, located.action.quote);
    assert_eq!(cited, "il faut migrer la CI vers les runners ARM");
    assert_eq!(delegations(&pool, submitted.run_id.0).await, 0);
}

#[tokio::test]
async fn an_abandoned_run_resumes_after_its_last_finished_step() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let first = FakeRuntime::default();
    let claimed = usecase
        .claim_run(submitted.run_id, Utc::now(), None)
        .await
        .unwrap()
        .unwrap();
    assert!(!claimed.resumed);
    let pipeline = MeetingPipeline::new(usecase.clone(), first.clone(), FakeExchanger, config());
    let busy = pipeline.execute(submitted.run_id, None).await;
    assert!(busy.is_ok(), "a fresh heartbeat keeps others away");
    assert_eq!(first.extractions.load(Ordering::SeqCst), 0);

    sqlx::query(
        "INSERT INTO run_steps (id, run_id, organization_id, seq, kind, status, attempt, output, started_at, finished_at)
         VALUES ($1, $2, $3, 0, 'extract', 'succeeded', 1, $4, now(), now())",
    )
    .bind(Uuid::now_v7())
    .bind(submitted.run_id.0)
    .bind(organization_id.0)
    .bind(serde_json::json!({ "actions": [{
        "action": { "title": "Migrer", "description": "", "quote": "migrer la CI", "owner": null, "due": null },
        "start": 17, "end": 29
    }], "unlocated": [] }))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE runs SET heartbeat_at = now() - interval '1 hour' WHERE id = $1")
        .bind(submitted.run_id.0)
        .execute(&pool)
        .await
        .unwrap();

    let second = FakeRuntime::default();
    MeetingPipeline::new(usecase.clone(), second.clone(), FakeExchanger, config())
        .execute(submitted.run_id, None)
        .await
        .unwrap();

    assert_eq!(
        second.extractions.load(Ordering::SeqCst),
        0,
        "the extraction is not redone"
    );
    assert_eq!(
        second.writes.lock().unwrap()[0].actions[0].action.title,
        "Migrer"
    );
    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
}

#[tokio::test]
async fn without_a_delegation_the_run_fails_and_says_why() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = usecase
        .submit_transcript(SubmitTranscript {
            subject: Subject {
                id: "user-1".into(),
                organization_id,
                actor: None,
            },
            title: "Rétro".into(),
            body: BODY.into(),
        })
        .await
        .unwrap();

    MeetingPipeline::new(
        usecase.clone(),
        FakeRuntime::default(),
        FakeExchanger,
        config(),
    )
    .execute(submitted.run_id, None)
    .await
    .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "failed");
    let failure: String = sqlx::query_scalar("SELECT failure FROM runs WHERE id = $1")
        .bind(submitted.run_id.0)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(failure.contains("no delegation"), "{failure}");
}
