#![allow(clippy::unwrap_used)]

use capitainerie_common::{CardId, CoreError, OrganizationId, RunId};
use capitainerie_core::{
    AgentDelegation, CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::{
            Card, Column,
            service::{CreateCard, MoveCard},
        },
        delegation::{DelegatedToken, ports::TokenExchanger},
        execution::{EXECUTOR_AGENT, ExecutorModel, TaskGrant},
        provider::ModelAssignments,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

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
    let pool = postgres::connect(
        &database_url().or_else(|| {
            eprintln!("skipped: DATABASE_URL is unset");
            None
        })?,
        3,
    )
    .await
    .unwrap();
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
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap());
    Some((pool, usecase, OrganizationId(id)))
}

fn human(organization_id: OrganizationId, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin,
    }
}

fn agent(organization_id: OrganizationId) -> Subject {
    Subject {
        actor: Some(EXECUTOR_AGENT.into()),
        ..human(organization_id, false)
    }
}

async fn todo_card(usecase: &CapitainerieUseCase, organization_id: OrganizationId) -> Card {
    let proposed = usecase
        .create_card(
            human(organization_id, false),
            CreateCard {
                title: "Ajouter un CHANGELOG".into(),
                description: "À la racine".into(),
                provenance: None,
                run_id: None,
            },
        )
        .await
        .unwrap();
    usecase
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
        .unwrap()
}

async fn connect(usecase: &CapitainerieUseCase, organization_id: OrganizationId) {
    usecase
        .connect_github(human(organization_id, true), 42, "acme".into())
        .await
        .unwrap();
}

async fn execute(
    usecase: &CapitainerieUseCase,
    subject: Subject,
    card: &Card,
) -> Result<capitainerie_core::StartedExecution, CoreError> {
    usecase
        .execute_card_delegated(
            subject,
            card.id,
            "acme/web".parse().unwrap(),
            "main".into(),
            card.version,
            AgentDelegation {
                exchanger: &FakeExchanger,
                issuer: "http://idp.test",
                user_token: &SecretToken::new("user"),
                agents: &[],
            },
        )
        .await
}

async fn run_count(pool: &PgPool, card_id: CardId) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM runs WHERE card_id = $1")
        .bind(card_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn random_hash() -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash[..16].copy_from_slice(Uuid::now_v7().as_bytes());
    hash[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    hash
}

fn conflict(error: CoreError) -> String {
    match error {
        CoreError::Conflict(code) => code,
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[tokio::test]
async fn a_member_hands_a_validated_card_to_the_executor() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let card = todo_card(&usecase, organization_id).await;

    let started = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap();

    assert_eq!(started.card.column, Column::InProgress);
    let (status, pipeline, repository, base_branch, transcript): (
        String,
        Vec<String>,
        String,
        String,
        Option<Uuid>,
    ) = sqlx::query_as(
        "SELECT status, pipeline, repository, base_branch, transcript_id FROM runs WHERE id = $1",
    )
    .bind(started.run_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "queued");
    assert_eq!(pipeline, ["execute"]);
    assert_eq!(repository, "acme/web");
    assert_eq!(base_branch, "main");
    assert_eq!(transcript, None);

    let delegated: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM run_delegations WHERE run_id = $1 AND agent_client_id = $2",
    )
    .bind(started.run_id.0)
    .bind(EXECUTOR_AGENT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(delegated, 1);

    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM outbox_events WHERE partition_key = $1")
            .bind(started.run_id.0)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(kinds, ["run.queued"]);
    let moved: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE partition_key = $1 AND kind = 'card.moved'",
    )
    .bind(card.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(moved, 2);

    let overview = usecase
        .get_run(human(organization_id, false), started.run_id)
        .await
        .unwrap();
    assert_eq!(overview.title.as_deref(), Some("Ajouter un CHANGELOG"));
    assert_eq!(overview.run.subject.card_id(), Some(card.id));
}

#[tokio::test]
async fn the_run_freezes_the_organization_models() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let mut assignments = ModelAssignments::defaults("chat", "embed");
    assignments.executor = ExecutorModel::new("claude-opus-5").unwrap();
    sqlx::query(
        "INSERT INTO model_assignments (organization_id, assignments, updated_by, updated_at) VALUES ($1, $2, 'u', now())",
    )
    .bind(organization_id.0)
    .bind(serde_json::to_value(&assignments).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    let card = todo_card(&usecase, organization_id).await;

    let started = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap();

    let overview = usecase
        .get_run(human(organization_id, false), started.run_id)
        .await
        .unwrap();
    assert_eq!(overview.run.models, Some(assignments));
}

#[tokio::test]
async fn a_proposed_card_cannot_be_handed_over() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let proposed = usecase
        .create_card(
            human(organization_id, false),
            CreateCard {
                title: "Idée".into(),
                description: String::new(),
                provenance: None,
                run_id: None,
            },
        )
        .await
        .unwrap();

    let error = execute(&usecase, human(organization_id, false), &proposed)
        .await
        .unwrap_err();

    assert_eq!(conflict(error), "card_not_executable");
    assert_eq!(run_count(&pool, proposed.id).await, 0);
}

#[tokio::test]
async fn a_card_has_one_active_execution() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let card = todo_card(&usecase, organization_id).await;
    let first = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap();
    let back_to_todo = usecase
        .move_card(
            human(organization_id, false),
            card.id,
            MoveCard {
                to: Column::Todo,
                expected_version: first.card.version,
                run_id: None,
            },
        )
        .await
        .unwrap();

    let error = execute(&usecase, human(organization_id, false), &back_to_todo)
        .await
        .unwrap_err();

    assert_eq!(conflict(error), "execution_already_active");
    assert_eq!(run_count(&pool, card.id).await, 1);
}

#[tokio::test]
async fn the_database_refuses_two_active_executions_of_a_card() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let card = todo_card(&usecase, organization_id).await;
    let first = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap();

    let duplicate = sqlx::query(
        "INSERT INTO runs (id, organization_id, card_id, repository, base_branch, initiated_by, status, pipeline, created_at)
         VALUES ($1, $2, $3, 'acme/web', 'main', 'u', 'queued', '{execute}', now())",
    )
    .bind(Uuid::now_v7())
    .bind(organization_id.0)
    .bind(card.id.0)
    .execute(&pool)
    .await;
    assert!(duplicate.is_err());

    sqlx::query("UPDATE runs SET status = 'failed' WHERE id = $1")
        .bind(first.run_id.0)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO runs (id, organization_id, card_id, repository, base_branch, initiated_by, status, pipeline, created_at)
         VALUES ($1, $2, $3, 'acme/web', 'main', 'u', 'queued', '{execute}', now())",
    )
    .bind(Uuid::now_v7())
    .bind(organization_id.0)
    .bind(card.id.0)
    .execute(&pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn a_run_has_exactly_one_subject() {
    let Some((pool, _usecase, organization_id)) = setup().await else {
        return;
    };

    let neither = sqlx::query(
        "INSERT INTO runs (id, organization_id, initiated_by, status, pipeline, created_at)
         VALUES ($1, $2, 'u', 'queued', '{execute}', now())",
    )
    .bind(Uuid::now_v7())
    .bind(organization_id.0)
    .execute(&pool)
    .await;

    assert!(neither.is_err());
}

#[tokio::test]
async fn without_github_nothing_is_created() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let card = todo_card(&usecase, organization_id).await;

    let error = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap_err();

    assert_eq!(conflict(error), "github_not_connected");
    assert_eq!(run_count(&pool, card.id).await, 0);
    let cards = usecase
        .list_cards(human(organization_id, false))
        .await
        .unwrap();
    assert_eq!(cards[0].column, Column::Todo);
}

#[tokio::test]
async fn an_agent_cannot_hand_a_card_over() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let card = todo_card(&usecase, organization_id).await;

    let error = execute(&usecase, agent(organization_id), &card)
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Forbidden { .. }), "{error:?}");
    assert_eq!(run_count(&pool, card.id).await, 0);
}

#[tokio::test]
async fn a_stale_version_hands_nothing_over() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let mut card = todo_card(&usecase, organization_id).await;
    card.version -= 1;

    let error = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Conflict(_)));
    assert_eq!(run_count(&pool, card.id).await, 0);
}

#[tokio::test]
async fn a_card_of_another_organization_is_unknown() {
    let Some((_pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let card = todo_card(&usecase, organization_id).await;
    let other = OrganizationId::new();

    let error = execute(&usecase, human(other, false), &card)
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::NotFound), "{error:?}");
}

#[tokio::test]
async fn only_an_admin_connects_github() {
    let Some((_pool, usecase, organization_id)) = setup().await else {
        return;
    };

    let refused = usecase
        .connect_github(human(organization_id, false), 7, "acme".into())
        .await;
    let before = usecase
        .github_installation(human(organization_id, false))
        .await
        .unwrap();
    let connected = usecase
        .connect_github(human(organization_id, true), 7, "acme".into())
        .await
        .unwrap();
    let replaced = usecase
        .connect_github(human(organization_id, true), 8, "acme-inc".into())
        .await
        .unwrap();
    let read_by_member = usecase
        .github_installation(human(organization_id, false))
        .await
        .unwrap();
    let system_read = usecase
        .github_installation_for(organization_id)
        .await
        .unwrap();
    let member_disconnect = usecase
        .disconnect_github(human(organization_id, false))
        .await;
    usecase
        .disconnect_github(human(organization_id, true))
        .await
        .unwrap();
    let after = usecase
        .github_installation_for(organization_id)
        .await
        .unwrap();

    assert!(matches!(refused, Err(CoreError::Forbidden { .. })));
    assert_eq!(before, None);
    assert_eq!(connected.installation_id, 7);
    assert_eq!(read_by_member, Some(replaced.clone()));
    assert_eq!(read_by_member.unwrap().account_login, "acme-inc");
    assert_eq!(system_read, Some(replaced));
    assert!(matches!(
        member_disconnect,
        Err(CoreError::Forbidden { .. })
    ));
    assert_eq!(after, None);
}

#[tokio::test]
async fn a_denied_connection_is_audited() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };

    let _ = usecase
        .connect_github(human(organization_id, false), 7, "acme".into())
        .await;

    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'github.manage' AND NOT allowed",
    )
    .bind(organization_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denied, 1);
}

#[tokio::test]
async fn a_task_grant_is_found_by_hash_and_revoked_with_its_run() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let card = todo_card(&usecase, organization_id).await;
    let started = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap();
    let grant = TaskGrant {
        run_id: started.run_id,
        organization_id,
        repository: "acme/web".parse().unwrap(),
        branch: "capitainerie/0a1b2c3d".into(),
        expires_at: Utc::now() + chrono::Duration::minutes(40),
        revoked_at: None,
    };
    let hash = random_hash();

    usecase
        .create_task_grant(started.run_id, hash, grant.clone())
        .await
        .unwrap();
    let found = usecase.find_task_grant(hash).await.unwrap().unwrap();
    let unknown = usecase.find_task_grant(random_hash()).await.unwrap();
    usecase.revoke_task_grants(started.run_id).await.unwrap();
    usecase.revoke_task_grants(started.run_id).await.unwrap();
    let revoked = usecase.find_task_grant(hash).await.unwrap().unwrap();

    assert_eq!(found.repository, grant.repository);
    assert_eq!(found.branch, grant.branch);
    assert!(found.is_usable(Utc::now()));
    assert_eq!(unknown, None);
    assert!(revoked.revoked_at.is_some());
    assert!(!revoked.is_usable(Utc::now()));

    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT token_sha256 FROM task_grants WHERE run_id = $1")
            .bind(started.run_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, hash);
}

#[tokio::test]
async fn deleting_a_run_deletes_its_grants() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    connect(&usecase, organization_id).await;
    let card = todo_card(&usecase, organization_id).await;
    let started = execute(&usecase, human(organization_id, false), &card)
        .await
        .unwrap();
    let hash = random_hash();
    usecase
        .create_task_grant(
            started.run_id,
            hash,
            TaskGrant {
                run_id: RunId::new(),
                organization_id,
                repository: "acme/web".parse().unwrap(),
                branch: "capitainerie/x".into(),
                expires_at: Utc::now() + chrono::Duration::minutes(1),
                revoked_at: None,
            },
        )
        .await
        .unwrap();

    sqlx::query("DELETE FROM run_delegations WHERE run_id = $1")
        .bind(started.run_id.0)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM runs WHERE id = $1")
        .bind(started.run_id.0)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(usecase.find_task_grant(hash).await.unwrap(), None);
}
