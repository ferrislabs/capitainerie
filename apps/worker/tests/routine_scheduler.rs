#![allow(clippy::unwrap_used)]

use capitainerie_common::{CoreError, OrganizationId, RoutineId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::attributes::Priority,
        crew::{CrewAgent, service::CreateAgent},
        mcp::{
            CodeExchange, Discovery, RefreshExchange, TokenSet, ports::McpOAuth,
            service::CreateConnector,
        },
        provider::ProviderKind,
        routine::{IfMissed, IfOpen, schedule::ScheduleSpec, service::CreateRoutine},
        user_provider::service::PutUserProvider,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use chrono::{DateTime, Duration, SubsecRound, Utc};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

const REDIRECT: &str = "https://console.example/connectors/oauth/callback";
const BATCH: i64 = 1_000;

/// A tick claims every organization's due triggers: the tests take turns.
static TICKS: Mutex<()> = Mutex::const_new(());

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

struct FakeOAuth;

impl McpOAuth for FakeOAuth {
    async fn discover(&self, url: &str) -> Result<Discovery, CoreError> {
        Ok(Discovery {
            resource: url.to_owned(),
            issuer: "https://auth.example/realms/mestier".into(),
            authorization_endpoint: "https://auth.example/realms/mestier/auth".into(),
            token_endpoint: "https://auth.example/realms/mestier/token".into(),
            registration_endpoint: "https://auth.example/realms/mestier/register".into(),
            scopes: vec![],
        })
    }

    async fn register_client(&self, _: &str, _: &str, _: &[String]) -> Result<String, CoreError> {
        Ok("client-1".into())
    }

    async fn exchange_code(&self, _: &CodeExchange<'_>) -> Result<TokenSet, CoreError> {
        Ok(TokenSet {
            access_token: SecretToken::new("access"),
            refresh_token: None,
            expires_at: Some(Utc::now() + Duration::hours(1)),
            scope: None,
        })
    }

    async fn refresh(&self, _: &RefreshExchange<'_>) -> Result<TokenSet, CoreError> {
        Err(CoreError::Conflict("no refresh".into()))
    }
}

struct Fixture {
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization_id: OrganizationId,
}

#[derive(Debug, sqlx::FromRow)]
struct Passage {
    status: String,
    origin: String,
    scheduled_for: DateTime<Utc>,
    card_id: Option<Uuid>,
    run_id: Option<Uuid>,
    reason: Option<String>,
}

async fn fixture() -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 5).await.unwrap();
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(&slug)
    .bind(format!("http://idp.test/realms/{slug}"))
    .execute(&pool)
    .await
    .unwrap();
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap())
        .with_secrets_cipher(SecretCipher::new(&[2u8; 32]).unwrap());
    Some(Fixture {
        pool,
        usecase,
        organization_id: OrganizationId(id),
    })
}

impl Fixture {
    fn subject(&self) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: self.organization_id,
            actor: None,
            is_admin: false,
        }
    }

    async fn agent(&self, slug: &str, grants: &[&str]) -> CrewAgent {
        self.usecase
            .create_crew_agent(
                self.subject(),
                CreateAgent {
                    slug: slug.into(),
                    name: slug.into(),
                    role: String::new(),
                    description: String::new(),
                    model: String::new(),
                    hue: 0,
                    reports_to: None,
                    instructions: "Tu aides.".into(),
                    connector_grants: grants.iter().map(|grant| (*grant).to_owned()).collect(),
                },
            )
            .await
            .unwrap()
    }

    async fn connected(&self, slug: &str) {
        let id = self
            .usecase
            .create_mcp_connector(
                self.subject(),
                CreateConnector {
                    url: format!("https://{slug}.example/mcp"),
                    name: Some(slug.to_owned()),
                    slug: Some(slug.to_owned()),
                },
                REDIRECT,
                &FakeOAuth,
            )
            .await
            .unwrap()
            .id;
        let start = self
            .usecase
            .start_mcp_authorization(self.subject(), id, REDIRECT)
            .await
            .unwrap();
        self.usecase
            .complete_mcp_authorization(self.subject(), &start.state, "code", &FakeOAuth)
            .await
            .unwrap();
        self.usecase
            .put_user_provider(PutUserProvider {
                subject: self.subject(),
                kind: ProviderKind::Anthropic,
                secret: Some(SecretToken::new("sk-ant-api03-abcdefgh1234")),
                base_url: None,
                default_model: "claude-sonnet-5-5".into(),
                is_default: None,
            })
            .await
            .unwrap();
    }

    async fn routine(&self, agent: &CrewAgent, if_open: IfOpen, if_missed: IfMissed) -> RoutineId {
        self.usecase
            .create_routine(
                self.subject(),
                CreateRoutine {
                    slug: None,
                    name: "Revue des accès".into(),
                    prompt: "Passe en revue les accès externes.".into(),
                    agent_id: agent.id,
                    project_id: None,
                    priority: Priority::High,
                    title_template: None,
                    timezone: None,
                    enabled: true,
                    if_open,
                    if_missed,
                    allow_writes: true,
                    schedule: Some(ScheduleSpec {
                        frequency: "daily".into(),
                        time: Some("08:00".into()),
                        ..ScheduleSpec::default()
                    }),
                },
            )
            .await
            .unwrap()
            .routine
            .id
    }

    async fn due_at(&self, routine: RoutineId, at: DateTime<Utc>) {
        sqlx::query("UPDATE routine_triggers SET next_run_at = $2 WHERE routine_id = $1")
            .bind(routine.0)
            .bind(at)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn next_run_at(&self, routine: RoutineId) -> DateTime<Utc> {
        sqlx::query_scalar("SELECT next_run_at FROM routine_triggers WHERE routine_id = $1")
            .bind(routine.0)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn passages(&self, routine: RoutineId) -> Vec<Passage> {
        sqlx::query_as(
            "SELECT status, origin, scheduled_for, card_id, run_id, reason FROM routine_runs
             WHERE routine_id = $1 ORDER BY started_at, id",
        )
        .bind(routine.0)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn close_card(&self, card: Uuid) {
        sqlx::query("UPDATE cards SET \"column\" = 'done' WHERE id = $1")
            .bind(card)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn tick(&self, now: DateTime<Utc>) {
        self.usecase.tick_routines(now, BATCH).await.unwrap();
    }
}

fn now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(6)
}

#[tokio::test]
async fn a_due_routine_creates_its_card_and_starts_the_agent_run() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let _turn = TICKS.lock().await;
    fixture.connected("mestier").await;
    let agent = fixture.agent("hippolyte", &["mestier"]).await;
    let routine = fixture
        .routine(&agent, IfOpen::Skip, IfMissed::RunOnce)
        .await;
    let planned = fixture.next_run_at(routine).await;
    assert!(planned > Utc::now());

    let now = now();
    fixture.tick(now).await;
    assert!(fixture.passages(routine).await.is_empty(), "not due yet");

    let due = now - Duration::minutes(1);
    fixture.due_at(routine, due).await;
    fixture.tick(now).await;

    let passages = fixture.passages(routine).await;
    assert_eq!(passages.len(), 1, "{passages:?}");
    let passage = &passages[0];
    assert_eq!(passage.status, "created", "{passage:?}");
    assert_eq!(passage.origin, "planning");
    assert_eq!(passage.scheduled_for, due);
    assert_eq!(passage.reason, None);
    let (title, description, column, assignee, priority): (
        String,
        String,
        String,
        Option<Uuid>,
        String,
    ) = sqlx::query_as(
        "SELECT title, description, \"column\", assignee_agent_id, priority FROM cards WHERE id = $1",
    )
    .bind(passage.card_id.unwrap())
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert!(title.starts_with("Revue des accès · "), "{title}");
    assert_eq!(description, "Passe en revue les accès externes.");
    assert_eq!(column, "todo");
    assert_eq!(assignee, Some(agent.id.0));
    assert_eq!(priority, "high");
    let (initiated_by, allow_writes): (String, bool) = sqlx::query_as(
        "SELECT r.initiated_by, t.allow_writes FROM runs r JOIN agent_tasks t ON t.run_id = r.id
         WHERE r.id = $1",
    )
    .bind(passage.run_id.unwrap())
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(initiated_by, "user-1");
    assert!(allow_writes);
    assert!(fixture.next_run_at(routine).await > now);
    let last_run_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT last_run_at FROM routines WHERE id = $1")
            .bind(routine.0)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert!(last_run_at.is_some());
    let outbox: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM outbox_events WHERE organization_id = $1 AND topic = 'routines'",
    )
    .bind(fixture.organization_id.0)
    .fetch_all(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(outbox, ["routine.run_created"]);
}

#[tokio::test]
async fn a_refused_run_keeps_the_card_and_says_why() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let _turn = TICKS.lock().await;
    let agent = fixture.agent("agathe", &[]).await;
    let routine = fixture
        .routine(&agent, IfOpen::Skip, IfMissed::RunOnce)
        .await;
    let now = now();
    fixture.due_at(routine, now).await;
    fixture.tick(now).await;

    let passages = fixture.passages(routine).await;
    assert_eq!(passages.len(), 1);
    assert_eq!(passages[0].status, "created");
    assert!(passages[0].card_id.is_some());
    assert!(passages[0].run_id.is_none());
    assert!(
        passages[0]
            .reason
            .as_deref()
            .unwrap()
            .contains("exécution non lancée"),
        "{passages:?}"
    );
}

#[tokio::test]
async fn an_open_previous_card_skips_the_passage_when_if_open_is_skip() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let _turn = TICKS.lock().await;
    let agent = fixture.agent("agathe", &[]).await;
    let routine = fixture
        .routine(&agent, IfOpen::Skip, IfMissed::RunOnce)
        .await;
    let now = now();
    fixture.due_at(routine, now).await;
    fixture.tick(now).await;
    let later = now + Duration::days(1);
    fixture.due_at(routine, later).await;
    fixture.tick(later).await;

    let passages = fixture.passages(routine).await;
    assert_eq!(passages.len(), 2, "{passages:?}");
    assert_eq!(passages[1].status, "skipped");
    assert!(passages[1].card_id.is_none());
    assert!(
        passages[1]
            .reason
            .as_deref()
            .unwrap()
            .contains("encore ouverte")
    );
    assert!(fixture.next_run_at(routine).await > later);
    let cards: i64 = sqlx::query_scalar("SELECT count(*) FROM cards WHERE organization_id = $1")
        .bind(fixture.organization_id.0)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(cards, 1);

    fixture.close_card(passages[0].card_id.unwrap()).await;
    let after = later + Duration::days(1);
    fixture.due_at(routine, after).await;
    fixture.tick(after).await;
    assert_eq!(fixture.passages(routine).await[2].status, "created");
}

#[tokio::test]
async fn a_queued_passage_waits_for_the_previous_card_then_runs_once() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let _turn = TICKS.lock().await;
    let agent = fixture.agent("agathe", &[]).await;
    let routine = fixture.routine(&agent, IfOpen::Queue, IfMissed::Skip).await;
    let now = now();
    fixture.due_at(routine, now).await;
    fixture.tick(now).await;
    let later = now + Duration::days(1);
    fixture.due_at(routine, later).await;
    fixture.tick(later).await;
    fixture.tick(later + Duration::hours(1)).await;

    let passages = fixture.passages(routine).await;
    assert_eq!(passages.len(), 2, "{passages:?}");
    assert_eq!(passages[1].status, "queued");
    assert_eq!(fixture.next_run_at(routine).await, later);

    fixture.close_card(passages[0].card_id.unwrap()).await;
    let released = later + Duration::days(3);
    fixture.tick(released).await;
    let passages = fixture.passages(routine).await;
    assert_eq!(passages.len(), 2, "{passages:?}");
    assert_eq!(passages[1].status, "created");
    assert_eq!(passages[1].scheduled_for, later);
    assert!(fixture.next_run_at(routine).await > released);
}

#[tokio::test]
async fn a_missed_passage_is_skipped_run_once_or_caught_up() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let _turn = TICKS.lock().await;
    let agent = fixture.agent("agathe", &[]).await;
    let now = now();
    let missed = now - Duration::days(3);

    let skip = fixture
        .routine(&agent, IfOpen::Always, IfMissed::Skip)
        .await;
    fixture.due_at(skip, missed).await;
    let run_once = fixture
        .usecase
        .create_routine(
            fixture.subject(),
            CreateRoutine {
                slug: Some("une-fois".into()),
                name: "Une fois".into(),
                prompt: "x".into(),
                agent_id: agent.id,
                project_id: None,
                priority: Priority::Normal,
                title_template: None,
                timezone: None,
                enabled: true,
                if_open: IfOpen::Always,
                if_missed: IfMissed::RunOnce,
                allow_writes: false,
                schedule: Some(ScheduleSpec {
                    frequency: "daily".into(),
                    time: Some("08:00".into()),
                    ..ScheduleSpec::default()
                }),
            },
        )
        .await
        .unwrap()
        .routine
        .id;
    fixture.due_at(run_once, missed).await;
    let run_all = fixture
        .usecase
        .create_routine(
            fixture.subject(),
            CreateRoutine {
                slug: Some("toutes".into()),
                name: "Toutes".into(),
                prompt: "x".into(),
                agent_id: agent.id,
                project_id: None,
                priority: Priority::Normal,
                title_template: None,
                timezone: Some("UTC".into()),
                enabled: true,
                if_open: IfOpen::Always,
                if_missed: IfMissed::RunAll,
                allow_writes: false,
                schedule: Some(ScheduleSpec {
                    frequency: "every_hours".into(),
                    hours: Some(24),
                    ..ScheduleSpec::default()
                }),
            },
        )
        .await
        .unwrap()
        .routine
        .id;
    fixture.due_at(run_all, missed).await;

    fixture.tick(now).await;

    let skipped = fixture.passages(skip).await;
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].status, "skipped");
    assert!(skipped[0].reason.as_deref().unwrap().contains("manqué"));
    assert!(fixture.next_run_at(skip).await > now);

    let once = fixture.passages(run_once).await;
    assert_eq!(once.len(), 1);
    assert_eq!(once[0].status, "created");
    assert_eq!(once[0].scheduled_for, missed);
    assert!(fixture.next_run_at(run_once).await > now);

    assert_eq!(fixture.passages(run_all).await.len(), 1);
    assert_eq!(
        fixture.next_run_at(run_all).await,
        missed + Duration::days(1)
    );
    fixture.tick(now).await;
    fixture.tick(now).await;
    let caught_up = fixture.passages(run_all).await;
    assert_eq!(caught_up.len(), 3, "{caught_up:?}");
    assert!(caught_up.iter().all(|passage| passage.status == "created"));
    assert_eq!(
        fixture.next_run_at(run_all).await,
        missed + Duration::days(3)
    );
}

#[tokio::test]
async fn two_concurrent_ticks_run_a_passage_once() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let _turn = TICKS.lock().await;
    let agent = fixture.agent("agathe", &[]).await;
    let routine = fixture
        .routine(&agent, IfOpen::Always, IfMissed::RunOnce)
        .await;
    let now = now();
    fixture.due_at(routine, now).await;
    let other = CapitainerieUseCase::with_local_policies(fixture.pool.clone())
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap())
        .with_secrets_cipher(SecretCipher::new(&[2u8; 32]).unwrap());

    let (first, second) = tokio::join!(
        fixture.usecase.tick_routines(now, BATCH),
        other.tick_routines(now, BATCH),
    );
    first.unwrap();
    second.unwrap();

    let passages = fixture.passages(routine).await;
    assert_eq!(passages.len(), 1, "{passages:?}");
    assert_eq!(passages[0].status, "created");
}
