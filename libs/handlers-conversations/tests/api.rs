#![allow(clippy::unwrap_used)]

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{Extension, Router};
use capitainerie_auth::Identity;
use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        conversation::{
            ProposedCard, ToolRecord,
            turn::{
                CaptainRuntime, LocalTools, TurnEnd, TurnEvent, TurnRequest, TurnSink, TurnState,
            },
        },
        delegation::{DelegatedToken, ports::TokenExchanger},
        execution::{DraftPullRequest, PullRequestUrl, RepositoryRef, ports::CodeHost},
        provider::{
            ModelAssignments, ModelRef, ProviderAccess, ProviderKind,
            ports::{CatalogModel, ModelCatalog},
            service::PutProvider,
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::Caller;
use capitainerie_handlers_conversations::{ConversationsState, router};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{net::TcpListener, sync::Notify};
use uuid::Uuid;

const MEMBER_TOKEN: &str = "the-members-own-token-9f3a";

enum Act {
    Event(TurnEvent),
    Usage(u64, u64),
    Wait,
    Fail(&'static str),
    Local(&'static str),
    UserMessageIsStored,
}

#[derive(Clone, Default)]
struct FakeRuntime {
    scripts: Arc<Mutex<VecDeque<Vec<Act>>>>,
    requests: Arc<Mutex<Vec<TurnRequest>>>,
    locals: Arc<Mutex<Vec<Result<String, String>>>>,
    stored_before_model: Arc<Mutex<Vec<bool>>>,
    entered: Arc<Notify>,
    gate: Arc<Notify>,
    pool: Option<PgPool>,
}

impl FakeRuntime {
    fn scripted(pool: &PgPool, scripts: Vec<Vec<Act>>) -> Self {
        Self {
            scripts: Arc::new(Mutex::new(scripts.into())),
            pool: Some(pool.clone()),
            ..Self::default()
        }
    }

    fn requests(&self) -> Vec<TurnRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl CaptainRuntime for FakeRuntime {
    async fn run_turn<L: LocalTools, S: TurnSink>(
        &self,
        request: &TurnRequest,
        local: &L,
        sink: &S,
        state: &mut TurnState,
    ) -> Result<TurnEnd, CoreError> {
        self.requests.lock().unwrap().push(request.clone());
        let script = self.scripts.lock().unwrap().pop_front().unwrap_or_default();
        for act in script {
            match act {
                Act::Event(event) => {
                    match &event {
                        TurnEvent::Delta(text) => state.text.push_str(text),
                        TurnEvent::ToolFinished {
                            name,
                            succeeded,
                            latency_ms,
                            summary,
                            ..
                        } => state.tool_calls.push(ToolRecord {
                            name: name.clone(),
                            arguments: "{}".into(),
                            succeeded: *succeeded,
                            latency_ms: *latency_ms,
                            summary: summary.clone(),
                        }),
                        TurnEvent::Card(card) => state.cards.push(card.clone()),
                        TurnEvent::ToolStarted { .. } => {}
                    }
                    if !sink.emit(event).await {
                        return Ok(TurnEnd::Interrupted);
                    }
                }
                Act::Usage(input, output) => {
                    state.usage.input_tokens += input;
                    state.usage.output_tokens += output;
                }
                Act::Wait => {
                    self.entered.notify_one();
                    self.gate.notified().await;
                }
                Act::Fail(message) => return Err(CoreError::Internal(message.into())),
                Act::Local(name) => {
                    let result = local.call(name, &json!({})).await;
                    self.locals.lock().unwrap().push(result);
                }
                Act::UserMessageIsStored => {
                    let pool = self.pool.as_ref().unwrap();
                    let last = request.history.last().unwrap().text.clone();
                    let stored: i64 = sqlx::query_scalar(
                        "SELECT count(*) FROM conversation_messages WHERE role = 'user' AND content->>'text' = $1",
                    )
                    .bind(last)
                    .fetch_one(pool)
                    .await
                    .unwrap();
                    self.stored_before_model.lock().unwrap().push(stored == 1);
                }
            }
        }
        Ok(TurnEnd::Completed)
    }
}

#[derive(Clone, Default)]
struct FakeExchanger {
    seen: Arc<Mutex<Vec<(String, String)>>>,
    refuse: bool,
}

impl TokenExchanger for FakeExchanger {
    async fn for_agent(
        &self,
        _: &str,
        user_token: &SecretToken,
        agent_client_id: &str,
    ) -> Result<DelegatedToken, CoreError> {
        self.seen.lock().unwrap().push((
            "for_agent".into(),
            format!("{}|{agent_client_id}", user_token.expose()),
        ));
        if self.refuse {
            return Err(CoreError::Forbidden {
                reason: Some("access_denied".into()),
            });
        }
        Ok(DelegatedToken {
            token: SecretToken::new("hop-one"),
            expires_at: chrono_now(),
        })
    }

    async fn for_mcp(
        &self,
        _: &str,
        agent_token: &SecretToken,
        agent_client_id: &str,
    ) -> Result<DelegatedToken, CoreError> {
        self.seen.lock().unwrap().push((
            "for_mcp".into(),
            format!("{}|{agent_client_id}", agent_token.expose()),
        ));
        Ok(DelegatedToken {
            token: SecretToken::new("delegated-to-mcp"),
            expires_at: chrono_now(),
        })
    }
}

fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    std::time::SystemTime::now().into()
}

struct FakeHost;

impl CodeHost for FakeHost {
    async fn repositories(
        &self,
        _: OrganizationId,
        installation_id: i64,
    ) -> Result<Vec<RepositoryRef>, CoreError> {
        assert!(installation_id > 0);
        Ok(vec![
            "acme/site".parse().unwrap(),
            "acme/api".parse().unwrap(),
        ])
    }

    async fn branch_head(
        &self,
        _: OrganizationId,
        _: i64,
        _: &RepositoryRef,
        _: &str,
    ) -> Result<Option<String>, CoreError> {
        Err(CoreError::Internal("unused".into()))
    }

    async fn open_draft_pr(
        &self,
        _: OrganizationId,
        _: i64,
        _: DraftPullRequest,
    ) -> Result<PullRequestUrl, CoreError> {
        Err(CoreError::Internal("unused".into()))
    }
}

struct FakeCatalog;

impl ModelCatalog for FakeCatalog {
    async fn list_models(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        Ok(vec![CatalogModel {
            provider: access.kind,
            id: "mistral-small".into(),
            display_name: "mistral-small".into(),
            chat: true,
            embeddings: true,
            transcription: false,
            ocr: false,
            context_window: None,
            input_price_per_mtok_micros: None,
            output_price_per_mtok_micros: None,
        }])
    }
}

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

async fn pool() -> Option<PgPool> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    Some(postgres::connect(&url, 6).await.unwrap())
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
        created_at: std::time::SystemTime::now().into(),
    }
}

fn caller_as(organization: &Organization, subject: &str, actor: Option<&str>) -> Caller {
    Caller {
        identity: Identity {
            subject: subject.into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: actor.map(|client_id| capitainerie_auth::Actor {
                sub: "agent".into(),
                client_id: Some(client_id.to_owned()),
                act: None,
            }),
            roles: vec!["capitainerie-admin".into()],
        },
        organization: organization.clone(),
        token: SecretToken::new(MEMBER_TOKEN),
    }
}

fn caller(organization: &Organization) -> Caller {
    caller_as(organization, "user-1", None)
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[9u8; 32]).unwrap())
}

async fn configure_captain(pool: &PgPool, organization: &Organization) {
    let usecase = usecase(pool);
    let subject = caller(organization).subject();
    usecase
        .put_provider(PutProvider {
            subject: subject.clone(),
            kind: ProviderKind::Mistral,
            secret: SecretToken::new("mistral-secret-key"),
            base_url: None,
        })
        .await
        .unwrap();
    let mut assignments = ModelAssignments::defaults("mistral-small", "mistral-small");
    let chat = ModelRef {
        provider: ProviderKind::Mistral,
        model: "mistral-small".into(),
    };
    assignments.extractor = chat.clone();
    assignments.judge = chat.clone();
    assignments.writer = chat;
    assignments.embeddings.provider = ProviderKind::Mistral;
    usecase
        .put_model_assignments(subject, assignments, &FakeCatalog)
        .await
        .unwrap();
}

struct Server {
    base: String,
    runtime: FakeRuntime,
    exchanger: FakeExchanger,
}

async fn serve(
    pool: &PgPool,
    caller: Caller,
    runtime: FakeRuntime,
    exchanger: Option<FakeExchanger>,
) -> Server {
    let seen = exchanger.clone().unwrap_or_default();
    let state = ConversationsState::new(
        usecase(pool),
        runtime.clone(),
        Arc::new(FakeHost),
        exchanger,
    );
    let app: Router = router::<(), _, _, _>(state).layer(Extension(caller));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Server {
        base: format!("http://{address}/api/v1/conversations"),
        runtime,
        exchanger: seen,
    }
}

impl Server {
    async fn create(&self, message: &str) -> String {
        let response = reqwest::Client::new()
            .post(&self.base)
            .json(&json!({ "message": message }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
        let body: Value = response.json().await.unwrap();
        body["id"].as_str().unwrap().to_owned()
    }

    async fn turn(&self, id: &str, body: Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/{id}/messages", self.base))
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    async fn get(&self, id: &str) -> (u16, Value) {
        let response = reqwest::get(format!("{}/{id}", self.base)).await.unwrap();
        (
            response.status().as_u16(),
            response.json().await.unwrap_or(Value::Null),
        )
    }
}

fn events_of(body: &str) -> Vec<(String, Value)> {
    body.split("\n\n")
        .filter_map(|block| {
            let mut name = None;
            let mut data = None;
            for line in block.lines() {
                if let Some(value) = line.strip_prefix("event:") {
                    name = Some(value.trim().to_owned());
                } else if let Some(value) = line.strip_prefix("data:") {
                    data = serde_json::from_str::<Value>(value.trim()).ok();
                }
            }
            Some((name?, data?))
        })
        .collect()
}

async fn wait_for<F: AsyncFn() -> bool>(what: &str, check: F) {
    for _ in 0..100 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for {what}");
}

fn delta(text: &str) -> Act {
    Act::Event(TurnEvent::Delta(text.into()))
}

#[tokio::test]
async fn a_turn_streams_delta_tool_card_then_done_in_order_and_is_stored() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let card = ProposedCard {
        card_id: Uuid::from_u128(5),
        title: "Ajouter un README".into(),
    };
    let runtime = FakeRuntime::scripted(
        &pool,
        vec![vec![
            Act::UserMessageIsStored,
            delta("Je regarde."),
            Act::Event(TurnEvent::ToolStarted {
                id: "t1".into(),
                name: "search_cards".into(),
                arguments: "{}".into(),
            }),
            Act::Event(TurnEvent::ToolFinished {
                id: "t1".into(),
                name: "search_cards".into(),
                succeeded: true,
                latency_ms: 12,
                summary: "aucune carte".into(),
            }),
            Act::Event(TurnEvent::Card(card.clone())),
            delta(" Voilà."),
            Act::Usage(120, 30),
        ]],
    );
    let server = serve(
        &pool,
        caller(&organization),
        runtime,
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server
        .create(&format!("Ajoute un README pour Acme {}", Uuid::now_v7()))
        .await;

    let response = server.turn(&id, json!({})).await;

    assert_eq!(response.status(), 200);
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
    let events = events_of(&response.text().await.unwrap());
    let names: Vec<_> = events.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["delta", "tool", "tool", "card", "delta", "done"]);
    assert_eq!(events[0].1["text"], "Je regarde.");
    assert_eq!(events[1].1["phase"], "start");
    assert_eq!(events[2].1["phase"], "end");
    assert_eq!(events[2].1["summary"], "aucune carte");
    assert_eq!(events[3].1["card_id"], card.card_id.to_string());
    assert_eq!(events[5].1["status"], "complete");
    assert_eq!(events[5].1["usage"]["input_tokens"], 120);
    assert_eq!(*server.runtime.stored_before_model.lock().unwrap(), [true]);

    let (status, stored) = server.get(&id).await;
    assert_eq!(status, 200);
    let messages = stored["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["status"], "complete");
    assert_eq!(messages[1]["text"], "Je regarde. Voilà.");
    assert_eq!(messages[1]["tool_calls"][0]["summary"], "aucune carte");
    assert_eq!(messages[1]["cards"][0]["title"], "Ajouter un README");
    assert_eq!(stored["conversation"]["input_tokens"], 120);
    assert_eq!(stored["conversation"]["model"], "mistral-small");
}

#[tokio::test]
async fn the_members_token_is_exchanged_and_never_reaches_the_runtime_or_the_database() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let runtime = FakeRuntime::scripted(&pool, vec![vec![delta("Ok.")]]);
    let server = serve(
        &pool,
        caller(&organization),
        runtime,
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server.create("Bonjour").await;

    server.turn(&id, json!({})).await.text().await.unwrap();

    assert_eq!(
        *server.exchanger.seen.lock().unwrap(),
        [
            (
                "for_agent".to_owned(),
                format!("{MEMBER_TOKEN}|capitainerie-agent-captain")
            ),
            (
                "for_mcp".to_owned(),
                "hop-one|capitainerie-agent-captain".to_owned()
            ),
        ]
    );
    let requests = server.runtime.requests();
    assert_eq!(requests[0].mcp_token.expose(), "delegated-to-mcp");
    assert_eq!(requests[0].limits.max_tool_calls, 20);
    assert_eq!(requests[0].limits.max_output_tokens, 16_000);
    assert!(!format!("{:?}", requests[0]).contains(MEMBER_TOKEN));
    let leaked: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM conversation_messages m JOIN conversations c ON c.id = m.conversation_id
                 WHERE c.organization_id = $1 AND (m.content::text LIKE '%' || $2 || '%' OR m.content::text LIKE '%delegated-to-mcp%'))
              + (SELECT count(*) FROM conversations WHERE organization_id = $1 AND (title LIKE '%' || $2 || '%' OR model::text LIKE '%' || $2 || '%'))",
    )
    .bind(organization.id.0)
    .bind(MEMBER_TOKEN)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leaked, 0);
}

#[tokio::test]
async fn another_member_or_organization_gets_a_404_everywhere() {
    let Some(pool) = pool().await else { return };
    let organization_a = organization(&pool).await;
    let organization_b = organization(&pool).await;
    configure_captain(&pool, &organization_a).await;
    let owner = serve(
        &pool,
        caller(&organization_a),
        FakeRuntime::default(),
        Some(FakeExchanger::default()),
    )
    .await;
    let id = owner.create("Secret").await;

    for stranger in [
        caller_as(&organization_a, "user-2", None),
        caller_as(&organization_b, "user-1", None),
    ] {
        let server = serve(
            &pool,
            stranger,
            FakeRuntime::default(),
            Some(FakeExchanger::default()),
        )
        .await;
        assert_eq!(server.get(&id).await.0, 404);
        let patch = reqwest::Client::new()
            .patch(format!("{}/{id}", server.base))
            .json(&json!({ "title": "pris" }))
            .send()
            .await
            .unwrap();
        assert_eq!(patch.status(), 404);
        assert_eq!(
            server
                .turn(&id, json!({ "message": "salut" }))
                .await
                .status(),
            404
        );
        let listed: Value = reqwest::get(&server.base)
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(listed["conversations"].as_array().unwrap().is_empty());
        assert!(server.runtime.requests().is_empty());
    }
}

#[tokio::test]
async fn a_second_turn_while_the_first_runs_is_a_409_and_the_slot_frees_afterwards() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let runtime = FakeRuntime::scripted(
        &pool,
        vec![
            vec![delta("Début"), Act::Wait, delta(" fin")],
            vec![delta("Autre")],
        ],
    );
    let server = serve(
        &pool,
        caller(&organization),
        runtime,
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server.create("Bonjour").await;

    let first = server.turn(&id, json!({})).await;
    server.runtime.entered.notified().await;
    let second = server.turn(&id, json!({ "message": "Encore" })).await;

    assert_eq!(second.status(), 409);
    let refused: Value = second.json().await.unwrap();
    assert_eq!(refused["code"], "turn_in_progress");
    server.runtime.gate.notify_one();
    let events = events_of(&first.text().await.unwrap());
    assert_eq!(events.last().unwrap().0, "done");
    let third = server.turn(&id, json!({ "message": "Encore" })).await;
    assert_eq!(third.status(), 200);
    third.text().await.unwrap();
    let (_, stored) = server.get(&id).await;
    let roles: Vec<_> = stored["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["role"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(roles, ["user", "assistant", "user", "assistant"]);
}

#[tokio::test]
async fn a_client_that_leaves_mid_turn_leaves_an_interrupted_answer_and_frees_the_slot() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let runtime = FakeRuntime::scripted(
        &pool,
        vec![vec![
            delta("Je cher"),
            Act::Usage(50, 4),
            Act::Wait,
            delta("che la suite"),
        ]],
    );
    let server = serve(
        &pool,
        caller(&organization),
        runtime,
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server.create("Bonjour").await;

    let mut response = server.turn(&id, json!({})).await;
    let first = response.chunk().await.unwrap().unwrap();
    assert!(String::from_utf8_lossy(&first).contains("Je cher"));
    server.runtime.entered.notified().await;
    drop(response);

    let probe = id.clone();
    let base = server.base.clone();
    wait_for("the interrupted answer", async || {
        let stored: Value = reqwest::get(format!("{base}/{probe}"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        stored["messages"].as_array().is_some_and(|messages| {
            messages.iter().any(|m| {
                m["role"] == "assistant" && m["status"] == "interrupted" && m["text"] == "Je cher"
            })
        })
    })
    .await;
    let (_, stored) = server.get(&id).await;
    assert_eq!(stored["conversation"]["input_tokens"], 50);
    let again = server.turn(&id, json!({ "message": "Reprends" })).await;
    assert_eq!(again.status(), 200);
}

#[tokio::test]
async fn a_provider_failure_ends_with_an_error_event_and_a_failed_message_keeping_the_text() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let runtime = FakeRuntime::scripted(
        &pool,
        vec![vec![delta("Début"), Act::Fail("quota exceeded")]],
    );
    let server = serve(
        &pool,
        caller(&organization),
        runtime,
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server.create("Bonjour").await;

    let events = events_of(&server.turn(&id, json!({})).await.text().await.unwrap());

    let last = events.last().unwrap();
    assert_eq!(last.0, "error");
    assert_eq!(last.1["code"], "provider_error");
    assert!(
        last.1["message"]
            .as_str()
            .unwrap()
            .contains("quota exceeded")
    );
    let (_, stored) = server.get(&id).await;
    let answer = &stored["messages"][1];
    assert_eq!(
        (answer["status"].as_str(), answer["text"].as_str()),
        (Some("failed"), Some("Début"))
    );
}

#[tokio::test]
async fn an_agent_is_refused_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let server = serve(
        &pool,
        caller_as(&organization, "user-1", Some("capitainerie-agent-captain")),
        FakeRuntime::default(),
        Some(FakeExchanger::default()),
    )
    .await;

    let created = reqwest::Client::new()
        .post(&server.base)
        .json(&json!({ "message": "Bonjour" }))
        .send()
        .await
        .unwrap();
    let turn = server
        .turn(&Uuid::now_v7().to_string(), json!({ "message": "x" }))
        .await;
    let listed = reqwest::get(&server.base).await.unwrap();

    assert_eq!(created.status(), 403);
    assert_eq!(turn.status(), 403);
    assert_eq!(listed.status(), 403);
    assert!(server.runtime.requests().is_empty());
    let denials: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND actor = 'capitainerie-agent-captain' AND NOT allowed AND action LIKE 'conversation.%'",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(denials >= 2, "{denials}");
}

#[tokio::test]
async fn without_a_captain_model_the_turn_is_a_409_and_the_message_is_not_stored() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let server = serve(
        &pool,
        caller(&organization),
        FakeRuntime::default(),
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server.create("Bonjour").await;

    let response = server.turn(&id, json!({ "message": "Encore" })).await;

    assert_eq!(response.status(), 409);
    assert_eq!(
        response.json::<Value>().await.unwrap()["code"],
        "captain_not_configured"
    );
    let (_, stored) = server.get(&id).await;
    assert_eq!(stored["messages"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn without_exchange_credentials_the_captain_is_unavailable() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let server = serve(&pool, caller(&organization), FakeRuntime::default(), None).await;
    let id = server.create("Bonjour").await;

    let response = server.turn(&id, json!({})).await;

    assert_eq!(response.status(), 409);
    assert_eq!(
        response.json::<Value>().await.unwrap()["code"],
        "captain_unavailable"
    );
}

#[tokio::test]
async fn a_refused_exchange_frees_the_slot_and_nothing_is_streamed() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let refusing = FakeExchanger {
        refuse: true,
        ..FakeExchanger::default()
    };
    let server = serve(
        &pool,
        caller(&organization),
        FakeRuntime::default(),
        Some(refusing),
    )
    .await;
    let id = server.create("Bonjour").await;

    let first = server.turn(&id, json!({})).await;
    let second = server.turn(&id, json!({})).await;

    assert_eq!(first.status(), 409);
    assert_eq!(second.status(), 409);
    assert_eq!(
        second.json::<Value>().await.unwrap()["code"],
        "captain_unavailable"
    );
}

#[tokio::test]
async fn a_conversation_over_its_token_limit_asks_for_a_new_one() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let server = serve(
        &pool,
        caller(&organization),
        FakeRuntime::default(),
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server.create("Bonjour").await;
    sqlx::query(
        "UPDATE conversations SET input_tokens = 399000, output_tokens = 1000 WHERE id = $1",
    )
    .bind(Uuid::parse_str(&id).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    let response = server.turn(&id, json!({ "message": "Encore" })).await;

    assert_eq!(response.status(), 409);
    assert_eq!(
        response.json::<Value>().await.unwrap()["code"],
        "conversation_full"
    );
    assert_eq!(server.get(&id).await.1["conversation"]["full"], true);
}

#[tokio::test]
async fn the_repository_tool_reads_the_installation_as_the_captain_and_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_captain(&pool, &organization).await;
    let runtime = FakeRuntime::scripted(
        &pool,
        vec![
            vec![Act::Local("list_repositories"), Act::Local("nope")],
            vec![Act::Local("list_repositories")],
        ],
    );
    let server = serve(
        &pool,
        caller(&organization),
        runtime,
        Some(FakeExchanger::default()),
    )
    .await;
    let id = server.create("Bonjour").await;

    server.turn(&id, json!({})).await.text().await.unwrap();
    usecase(&pool)
        .connect_github(
            caller(&organization).subject(),
            i64::from(Uuid::now_v7().as_u128() as u32) + 1,
            "acme".into(),
        )
        .await
        .unwrap();
    server
        .turn(&id, json!({ "message": "Encore" }))
        .await
        .text()
        .await
        .unwrap();

    let locals = server.runtime.locals.lock().unwrap().clone();
    assert!(matches!(&locals[0], Err(message) if message.contains("github_not_connected")));
    assert!(matches!(&locals[1], Err(message) if message.contains("unknown tool")));
    assert_eq!(locals[2], Ok("acme/site\nacme/api".to_owned()));
    let decisions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND actor = 'capitainerie-agent-captain' AND action = 'github.read' AND allowed",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(decisions, 2);
}

#[tokio::test]
async fn conversations_are_listed_renamed_and_archived() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let server = serve(
        &pool,
        caller(&organization),
        FakeRuntime::default(),
        Some(FakeExchanger::default()),
    )
    .await;
    let first = server.create("Premier fil").await;
    let second = server.create("Second fil").await;

    let listed: Value = reqwest::get(&server.base)
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let titles: Vec<_> = listed["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["title"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(titles, ["Second fil", "Premier fil"]);

    let patch = |id: &str, body: Value| {
        let url = format!("{}/{id}", server.base);
        async move {
            reqwest::Client::new()
                .patch(url)
                .json(&body)
                .send()
                .await
                .unwrap()
        }
    };
    let renamed = patch(&first, json!({ "title": "README Acme" })).await;
    assert_eq!(renamed.status(), 200);
    assert_eq!(
        renamed.json::<Value>().await.unwrap()["title"],
        "README Acme"
    );
    assert_eq!(patch(&first, json!({ "title": " " })).await.status(), 422);
    assert_eq!(
        patch(&second, json!({ "archived": true })).await.status(),
        200
    );

    let listed: Value = reqwest::get(&server.base)
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(listed["conversations"].as_array().unwrap().len(), 1);
    let empty = reqwest::Client::new()
        .post(&server.base)
        .json(&json!({ "message": "  " }))
        .send()
        .await
        .unwrap();
    assert_eq!(empty.status(), 422);
}
