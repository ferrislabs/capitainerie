#![allow(clippy::unwrap_used)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use capitainerie_common::{CardId, CoreError, OrganizationId};
use capitainerie_core::{
    AgentDelegation, CapitainerieUseCase, MeetingPipeline, PipelineConfig,
    domain::{
        agent::{
            ExtractedAction, Extraction, Judgement, LocatedAction, SecretToken, ToolCall, Usage,
            WriteReport, WriteRequest, ports::AgentRuntime,
        },
        authorization::Subject,
        delegation::{DelegatedToken, ports::TokenExchanger},
        provider::{
            ModelAssignments, ModelRef, ModelTarget, ProviderAccess, ProviderKind,
            service::PutProvider,
        },
        run::{service::SubmitTranscript, step::Pricing},
        similarity::{
            CardVector, Embedding, SimilarCard,
            ports::{Embedder, VectorStore},
        },
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
    two_actions: bool,
    judge: Option<bool>,
    judged: Arc<AtomicUsize>,
    targets: Arc<std::sync::Mutex<Vec<(&'static str, ModelTarget)>>>,
    echo_model: bool,
}

impl FakeRuntime {
    fn judging(same_task: bool) -> Self {
        Self {
            judge: Some(same_task),
            ..Self::default()
        }
    }

    fn with_two_located_actions() -> Self {
        Self {
            two_actions: true,
            ..Self::default()
        }
    }
}

fn two_located() -> Vec<ExtractedAction> {
    vec![
        ExtractedAction {
            title: "Migrer la CI vers ARM".into(),
            description: "Avant fin octobre".into(),
            quote: "migrer la CI vers les runners ARM".into(),
            owner: None,
            due: None,
        },
        ExtractedAction {
            title: "Prendre en charge".into(),
            description: String::new(),
            quote: "je m’en charge".into(),
            owner: Some("Malik".into()),
            due: None,
        },
    ]
}

impl AgentRuntime for FakeRuntime {
    async fn extract(
        &self,
        target: &ModelTarget,
        _transcript: &str,
    ) -> Result<Extraction, CoreError> {
        self.extractions.fetch_add(1, Ordering::SeqCst);
        self.targets
            .lock()
            .unwrap()
            .push(("extractor", target.clone()));
        let model = if self.echo_model {
            target.model.model.clone()
        } else {
            "fake".into()
        };
        if self.two_actions {
            return Ok(Extraction {
                actions: two_located(),
                usage: Usage {
                    model,
                    input_tokens: 1_000,
                    output_tokens: 200,
                },
            });
        }
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
                model,
                input_tokens: 1_000,
                output_tokens: 200,
            },
        })
    }

    async fn judge_duplicate(
        &self,
        target: &ModelTarget,
        _action: &LocatedAction,
        _candidate: &SimilarCard,
    ) -> Result<Judgement, CoreError> {
        self.judged.fetch_add(1, Ordering::SeqCst);
        self.targets.lock().unwrap().push(("judge", target.clone()));
        match self.judge {
            Some(same_task) => Ok(Judgement {
                same_task,
                reason: "the judge said so".into(),
                usage: Usage {
                    model: "fake-chat".into(),
                    input_tokens: 30,
                    output_tokens: 5,
                },
            }),
            None => Err(CoreError::Internal("the judge is down".into())),
        }
    }

    async fn write(
        &self,
        target: &ModelTarget,
        request: WriteRequest,
    ) -> Result<WriteReport, CoreError> {
        self.targets
            .lock()
            .unwrap()
            .push(("writer", target.clone()));
        let calls = request
            .duplicates
            .iter()
            .map(|duplicate| ToolCall {
                tool: "comment".into(),
                succeeded: true,
                latency_ms: 5,
                summary: format!("comment on card {}", duplicate.card_id),
            })
            .chain(request.actions.iter().map(|action| ToolCall {
                tool: "create_card".into(),
                succeeded: true,
                latency_ms: 5,
                summary: action.action.title.clone(),
            }))
            .collect();
        let request_has_actions = !request.actions.is_empty();
        self.writes.lock().unwrap().push(request);
        let usage = if request_has_actions {
            Usage {
                model: "fake".into(),
                input_tokens: 500,
                output_tokens: 100,
            }
        } else {
            Usage {
                model: "fake".into(),
                ..Usage::default()
            }
        };
        Ok(WriteReport {
            tool_calls: calls,
            usage,
        })
    }
}

/// Embeds a text mentioning the CI as `[1]` and any other as `[0]`, 10 tokens each.
#[derive(Clone, Default)]
struct FakeEmbedder {
    calls: Arc<AtomicUsize>,
    targets: Arc<std::sync::Mutex<Vec<ModelTarget>>>,
}

impl Embedder for FakeEmbedder {
    async fn embed(
        &self,
        target: &ModelTarget,
        texts: &[String],
    ) -> Result<(Vec<Embedding>, Usage), CoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.targets.lock().unwrap().push(target.clone());
        let embeddings = texts
            .iter()
            .map(|text| Embedding(vec![if text.contains("CI") { 1.0 } else { 0.0 }]))
            .collect();
        Ok((
            embeddings,
            Usage {
                model: "fake-embedding".into(),
                input_tokens: 10 * texts.len() as u64,
                output_tokens: 0,
            },
        ))
    }
}

/// Knows one card, found by whatever embeds as `[1]`.
#[derive(Clone)]
struct FakeVectors {
    known: SimilarCard,
    asked: Arc<std::sync::Mutex<Vec<OrganizationId>>>,
    models: Arc<std::sync::Mutex<Vec<String>>>,
}

impl FakeVectors {
    fn knowing(card_id: CardId, score: f32) -> Self {
        Self {
            known: SimilarCard {
                card_id,
                score,
                text: "Migrer la CI".into(),
            },
            asked: Arc::default(),
            models: Arc::default(),
        }
    }
}

impl VectorStore for FakeVectors {
    async fn upsert(&self, _model: &str, _card: CardVector) -> Result<(), CoreError> {
        Ok(())
    }

    async fn holds_cards(
        &self,
        _model: &str,
        _organization_id: OrganizationId,
    ) -> Result<bool, CoreError> {
        Ok(true)
    }

    async fn nearest(
        &self,
        model: &str,
        organization_id: OrganizationId,
        embedding: &Embedding,
        _limit: usize,
    ) -> Result<Vec<SimilarCard>, CoreError> {
        self.asked.lock().unwrap().push(organization_id);
        self.models.lock().unwrap().push(model.to_owned());
        Ok(if embedding.0[0] > 0.5 {
            vec![self.known.clone()]
        } else {
            Vec::new()
        })
    }
}

fn pipeline_for(
    usecase: &CapitainerieUseCase,
    runtime: &FakeRuntime,
    vectors: &FakeVectors,
) -> MeetingPipeline<FakeRuntime, FakeExchanger, FakeEmbedder, FakeVectors> {
    MeetingPipeline::new(
        usecase.clone(),
        runtime.clone(),
        FakeExchanger,
        FakeEmbedder::default(),
        vectors.clone(),
        config(),
    )
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
        dedup_threshold: 0.86,
        dedup_gray_floor: 0.65,
        default_models: ModelAssignments::defaults("env-chat", "env-embedding"),
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
                    is_admin: false,
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
    let vectors = FakeVectors::knowing(CardId::new(), 0.5);
    let pipeline = pipeline_for(&usecase, &runtime, &vectors);
    pipeline.execute(submitted.run_id, None).await.unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    assert_eq!(
        steps(&pool, submitted.run_id.0).await,
        [
            ("extract".to_owned(), "succeeded".to_owned(), 1_400),
            ("deduplicate".to_owned(), "succeeded".to_owned(), 10),
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
    let pipeline = pipeline_for(&usecase, &first, &FakeVectors::knowing(CardId::new(), 0.5));
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
    pipeline_for(&usecase, &second, &FakeVectors::knowing(CardId::new(), 0.5))
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
                is_admin: false,
            },
            title: "Rétro".into(),
            body: BODY.into(),
        })
        .await
        .unwrap();

    pipeline_for(
        &usecase,
        &FakeRuntime::default(),
        &FakeVectors::knowing(CardId::new(), 0.5),
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

async fn dedup_output(pool: &PgPool, run_id: Uuid) -> serde_json::Value {
    sqlx::query_scalar("SELECT output FROM run_steps WHERE run_id = $1 AND kind = 'deduplicate'")
        .bind(run_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_duplicate_is_commented_and_only_the_new_action_becomes_a_card() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let existing = CardId::new();
    let runtime = FakeRuntime::with_two_located_actions();
    let vectors = FakeVectors::knowing(existing, 0.93);

    pipeline_for(&usecase, &runtime, &vectors)
        .execute(submitted.run_id, None)
        .await
        .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    let output = dedup_output(&pool, submitted.run_id.0).await;
    assert_eq!(output["new"], serde_json::json!([1]));
    assert_eq!(output["duplicates"][0]["action"], 0);
    assert_eq!(output["duplicates"][0]["card_id"], existing.0.to_string());
    assert_eq!(output["embedding_tokens"], 20);
    assert!(
        vectors
            .asked
            .lock()
            .unwrap()
            .iter()
            .all(|asked| *asked == organization_id)
    );
    let writes = runtime.writes.lock().unwrap().clone();
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].actions.len(), 1);
    assert_eq!(writes[0].actions[0].action.title, "Prendre en charge");
    assert_eq!(writes[0].duplicates.len(), 1);
    assert_eq!(writes[0].duplicates[0].card_id, existing);
    assert_eq!(
        writes[0].duplicates[0].action.action.title,
        "Migrer la CI vers ARM"
    );
    let write_output: serde_json::Value =
        sqlx::query_scalar("SELECT output FROM run_steps WHERE run_id = $1 AND kind = 'write'")
            .bind(submitted.run_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    let tools: Vec<_> = write_output["tool_calls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|call| call["tool"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(tools, ["comment", "create_card"]);
}

#[tokio::test]
async fn when_every_action_is_a_duplicate_the_run_only_comments_and_succeeds() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let runtime = FakeRuntime::default();
    let vectors = FakeVectors::knowing(CardId::new(), 0.99);

    pipeline_for(&usecase, &runtime, &vectors)
        .execute(submitted.run_id, None)
        .await
        .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    let writes = runtime.writes.lock().unwrap().clone();
    assert_eq!(writes.len(), 1);
    assert!(
        writes[0].actions.is_empty(),
        "nothing is left for the model"
    );
    assert_eq!(writes[0].duplicates.len(), 1);
    assert_eq!(
        steps(&pool, submitted.run_id.0).await,
        [
            ("extract".to_owned(), "succeeded".to_owned(), 1_400),
            ("deduplicate".to_owned(), "succeeded".to_owned(), 10),
            ("write".to_owned(), "succeeded".to_owned(), 0),
        ]
    );
}

#[tokio::test]
async fn a_finished_deduplicate_step_is_reused_on_resume() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    usecase
        .claim_run(submitted.run_id, Utc::now(), None)
        .await
        .unwrap()
        .unwrap();
    let existing = CardId::new();
    let action = |title: &str, quote: &str, start: u32, end: u32| {
        serde_json::json!({
            "action": { "title": title, "description": "", "quote": quote, "owner": null, "due": null },
            "start": start, "end": end
        })
    };
    let steps_to_insert = [
        (
            0,
            "extract",
            serde_json::json!({ "actions": [
                action("Migrer", "migrer la CI", 17, 29),
                action("Autre", "je m’en charge", 70, 84)
            ], "unlocated": [] }),
        ),
        (
            1,
            "deduplicate",
            serde_json::json!({
                "new": [1],
                "duplicates": [{ "action": 0, "card_id": existing.0, "score": 0.7, "decided_by": "judge" }],
                "judged": [{ "action": 0, "card_id": existing.0, "score": 0.7, "same_task": true, "reason": "r" }],
                "embedding_tokens": 20
            }),
        ),
    ];
    for (seq, kind, output) in steps_to_insert {
        sqlx::query(
            "INSERT INTO run_steps (id, run_id, organization_id, seq, kind, status, attempt, output, started_at, finished_at)
             VALUES ($1, $2, $3, $4, $5, 'succeeded', 1, $6, now(), now())",
        )
        .bind(Uuid::now_v7())
        .bind(submitted.run_id.0)
        .bind(organization_id.0)
        .bind(seq)
        .bind(kind)
        .bind(output)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE runs SET heartbeat_at = now() - interval '1 hour' WHERE id = $1")
        .bind(submitted.run_id.0)
        .execute(&pool)
        .await
        .unwrap();
    let runtime = FakeRuntime::default();
    let embedder = FakeEmbedder::default();

    MeetingPipeline::new(
        usecase.clone(),
        runtime.clone(),
        FakeExchanger,
        embedder.clone(),
        FakeVectors::knowing(CardId::new(), 0.0),
        config(),
    )
    .execute(submitted.run_id, None)
    .await
    .unwrap();

    assert_eq!(embedder.calls.load(Ordering::SeqCst), 0, "no new embedding");
    assert_eq!(runtime.extractions.load(Ordering::SeqCst), 0);
    let writes = runtime.writes.lock().unwrap().clone();
    assert_eq!(writes[0].actions[0].action.title, "Autre");
    assert_eq!(writes[0].duplicates[0].card_id, existing);
    assert_eq!(writes[0].duplicates[0].action.action.title, "Migrer");
    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
}

async fn dedup_step(pool: &PgPool, run_id: Uuid) -> (Option<String>, i64, String) {
    sqlx::query_as(
        "SELECT agent, cost_micros, output->'judged'->0->>'reason' FROM run_steps WHERE run_id = $1 AND kind = 'deduplicate'",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_gray_zone_action_judged_the_same_task_becomes_a_comment_not_a_card() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let existing = CardId::new();
    let runtime = FakeRuntime::judging(true);
    let vectors = FakeVectors::knowing(existing, 0.7);

    pipeline_for(&usecase, &runtime, &vectors)
        .execute(submitted.run_id, None)
        .await
        .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    assert_eq!(runtime.judged.load(Ordering::SeqCst), 1);
    let output = dedup_output(&pool, submitted.run_id.0).await;
    assert_eq!(output["new"], serde_json::json!([]));
    assert_eq!(output["duplicates"][0]["card_id"], existing.0.to_string());
    assert_eq!(output["duplicates"][0]["decided_by"], "judge");
    assert_eq!(output["judged"][0]["same_task"], true);
    assert_eq!(output["judged"][0]["reason"], "the judge said so");
    assert_eq!(output["embedding_tokens"], 10);
    let (agent, cost, _) = dedup_step(&pool, submitted.run_id.0).await;
    assert_eq!(agent.as_deref(), Some("deduplicator"));
    assert_eq!(cost, 50, "embedding and judge tokens are both billed");
    let writes = runtime.writes.lock().unwrap().clone();
    assert!(writes[0].actions.is_empty());
    assert_eq!(writes[0].duplicates[0].card_id, existing);
}

#[tokio::test]
async fn a_gray_zone_action_judged_different_is_a_new_card() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let runtime = FakeRuntime::judging(false);

    pipeline_for(
        &usecase,
        &runtime,
        &FakeVectors::knowing(CardId::new(), 0.7),
    )
    .execute(submitted.run_id, None)
    .await
    .unwrap();

    let output = dedup_output(&pool, submitted.run_id.0).await;
    assert_eq!(output["new"], serde_json::json!([0]));
    assert_eq!(output["duplicates"], serde_json::json!([]));
    assert_eq!(output["judged"][0]["same_task"], false);
    let writes = runtime.writes.lock().unwrap().clone();
    assert_eq!(writes[0].actions.len(), 1);
    assert!(writes[0].duplicates.is_empty());
}

#[tokio::test]
async fn a_failing_judge_keeps_the_action_new_and_the_run_succeeds() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let runtime = FakeRuntime::default();

    pipeline_for(
        &usecase,
        &runtime,
        &FakeVectors::knowing(CardId::new(), 0.7),
    )
    .execute(submitted.run_id, None)
    .await
    .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    let output = dedup_output(&pool, submitted.run_id.0).await;
    assert_eq!(output["new"], serde_json::json!([0]));
    assert_eq!(
        output["judged"][0]["error"],
        "internal error: the judge is down"
    );
    let (_, cost, _) = dedup_step(&pool, submitted.run_id.0).await;
    assert_eq!(cost, 10);
}

#[tokio::test]
async fn outside_the_gray_zone_the_judge_is_never_asked() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    for score in [0.9, 0.5] {
        let submitted = submit(&usecase, organization_id).await;
        let runtime = FakeRuntime::judging(true);

        pipeline_for(
            &usecase,
            &runtime,
            &FakeVectors::knowing(CardId::new(), score),
        )
        .execute(submitted.run_id, None)
        .await
        .unwrap();

        assert_eq!(runtime.judged.load(Ordering::SeqCst), 0, "score {score}");
        assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    }
}

fn model(provider: ProviderKind, name: &str) -> ModelRef {
    ModelRef {
        provider,
        model: name.into(),
    }
}

fn claude_for_the_agents() -> ModelAssignments {
    ModelAssignments {
        extractor: model(ProviderKind::Anthropic, "claude-haiku-5-5"),
        judge: model(ProviderKind::Anthropic, "claude-sonnet-5-5"),
        writer: model(ProviderKind::Anthropic, "claude-haiku-5-5"),
        embeddings: model(ProviderKind::OpenAiCompatible, "bge-m3"),
        executor: Default::default(),
    }
}

async fn freeze_models(pool: &PgPool, run_id: Uuid, assignments: &ModelAssignments) {
    sqlx::query("UPDATE runs SET models = $2 WHERE id = $1")
        .bind(run_id)
        .bind(serde_json::to_value(assignments).unwrap())
        .execute(pool)
        .await
        .unwrap();
}

fn run_with(
    usecase: &CapitainerieUseCase,
    runtime: &FakeRuntime,
    embedder: &FakeEmbedder,
    vectors: &FakeVectors,
) -> MeetingPipeline<FakeRuntime, FakeExchanger, FakeEmbedder, FakeVectors> {
    MeetingPipeline::new(
        usecase.clone(),
        runtime.clone(),
        FakeExchanger,
        embedder.clone(),
        vectors.clone(),
        config(),
    )
}

#[tokio::test]
async fn a_run_without_models_uses_the_worker_defaults_on_its_own_endpoint() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    let runtime = FakeRuntime::judging(false);
    let embedder = FakeEmbedder::default();
    let vectors = FakeVectors::knowing(CardId::new(), 0.7);

    run_with(&usecase, &runtime, &embedder, &vectors)
        .execute(submitted.run_id, None)
        .await
        .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    let seen = runtime.targets.lock().unwrap().clone();
    let roles: Vec<_> = seen.iter().map(|(role, _)| *role).collect();
    assert_eq!(roles, ["extractor", "judge", "writer"]);
    for (_, target) in &seen {
        assert_eq!(
            target.model,
            model(ProviderKind::OpenAiCompatible, "env-chat")
        );
        assert_eq!(target.access, None);
    }
    let embedded = embedder.targets.lock().unwrap().clone();
    assert_eq!(
        embedded[0].model,
        model(ProviderKind::OpenAiCompatible, "env-embedding")
    );
    assert_eq!(embedded[0].access, None);
    assert_eq!(*vectors.models.lock().unwrap(), ["env-embedding"]);
}

#[tokio::test]
async fn a_run_with_claude_models_gets_the_key_of_its_organization() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let usecase = usecase.with_secrets_cipher(SecretCipher::new(&[7u8; 32]).unwrap());
    usecase
        .put_provider(PutProvider {
            subject: Subject {
                id: "admin".into(),
                organization_id,
                actor: None,
                is_admin: true,
            },
            kind: ProviderKind::Anthropic,
            secret: SecretToken::new("sk-ant-test-1234"),
            base_url: None,
        })
        .await
        .unwrap();
    let submitted = submit(&usecase, organization_id).await;
    freeze_models(&pool, submitted.run_id.0, &claude_for_the_agents()).await;
    let runtime = FakeRuntime {
        echo_model: true,
        ..FakeRuntime::judging(false)
    };
    let embedder = FakeEmbedder::default();
    let vectors = FakeVectors::knowing(CardId::new(), 0.7);

    run_with(&usecase, &runtime, &embedder, &vectors)
        .execute(submitted.run_id, None)
        .await
        .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "succeeded");
    let access = ProviderAccess {
        kind: ProviderKind::Anthropic,
        secret: SecretToken::new("sk-ant-test-1234"),
        base_url: None,
    };
    let seen = runtime.targets.lock().unwrap().clone();
    let by_role = |role: &str| {
        seen.iter()
            .find(|(name, _)| *name == role)
            .map(|(_, target)| target.clone())
            .unwrap()
    };
    assert_eq!(
        by_role("extractor").model,
        model(ProviderKind::Anthropic, "claude-haiku-5-5")
    );
    assert_eq!(by_role("extractor").access, Some(access.clone()));
    assert_eq!(
        by_role("judge").model,
        model(ProviderKind::Anthropic, "claude-sonnet-5-5")
    );
    assert_eq!(by_role("judge").access, Some(access.clone()));
    assert_eq!(by_role("writer").access, Some(access));
    let embedded = embedder.targets.lock().unwrap().clone();
    assert_eq!(embedded[0].model.model, "bge-m3");
    assert_eq!(
        embedded[0].access, None,
        "no key for this provider: the worker's endpoint"
    );
    assert_eq!(*vectors.models.lock().unwrap(), ["bge-m3"]);
    let extract_model: Option<String> =
        sqlx::query_scalar("SELECT model FROM run_steps WHERE run_id = $1 AND kind = 'extract'")
            .bind(submitted.run_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(extract_model.as_deref(), Some("claude-haiku-5-5"));
}

#[tokio::test]
async fn a_model_of_a_provider_without_a_key_fails_the_step_with_a_clear_message() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let submitted = submit(&usecase, organization_id).await;
    freeze_models(&pool, submitted.run_id.0, &claude_for_the_agents()).await;
    let runtime = FakeRuntime::default();

    run_with(
        &usecase,
        &runtime,
        &FakeEmbedder::default(),
        &FakeVectors::knowing(CardId::new(), 0.5),
    )
    .execute(submitted.run_id, None)
    .await
    .unwrap();

    assert_eq!(status(&pool, submitted.run_id.0).await, "failed");
    assert_eq!(runtime.extractions.load(Ordering::SeqCst), 0);
    let failure: String = sqlx::query_scalar("SELECT failure FROM runs WHERE id = $1")
        .bind(submitted.run_id.0)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        failure.contains("no anthropic key configured for this organization"),
        "{failure}"
    );
    assert_eq!(
        steps(&pool, submitted.run_id.0).await,
        [("extract".to_owned(), "failed".to_owned(), 0)]
    );
}
