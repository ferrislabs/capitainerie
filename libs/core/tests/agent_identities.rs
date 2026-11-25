#![allow(clippy::unwrap_used)]

use capitainerie_common::{AgentId, CoreError, OrganizationId};
use capitainerie_core::{
    domain::{
        agent::SecretToken,
        agent_identity::{
            AgentIdentity, IdentityState, IdentityStatus, ProvisionedClient,
            ports::AgentIdentityRepository,
        },
    },
    infrastructure::{
        agent_identity::postgres::PgAgentIdentityRepository, crypto::SecretCipher, postgres,
    },
};
use chrono::{Duration, DurationRound, Utc};
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

async fn pool() -> Option<PgPool> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    Some(postgres::connect(&url, 2).await.unwrap())
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

async fn agent(pool: &PgPool, organization_id: OrganizationId) -> AgentId {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO agents (id, organization_id, slug, name, archived, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, 'Bernard', false, 'test', now(), now())",
    )
    .bind(id)
    .bind(organization_id.0)
    .bind(format!("a-{}", id.simple()))
    .execute(pool)
    .await
    .unwrap();
    AgentId(id)
}

fn cipher() -> SecretCipher {
    SecretCipher::new(&[9; 32]).unwrap()
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
        .duration_trunc(Duration::microseconds(1))
        .unwrap()
}

fn client(client_uuid: &str) -> ProvisionedClient {
    ProvisionedClient {
        client_uuid: client_uuid.into(),
        secret: SecretToken::new("agent-secret"),
    }
}

async fn store(pool: &PgPool, identity: &AgentIdentity) -> Result<(), CoreError> {
    let cipher = cipher();
    postgres::with_tx(pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, Some(&cipher))
            .upsert(identity)
            .await
    })
    .await
}

async fn load(
    pool: &PgPool,
    organization_id: OrganizationId,
    agent_id: AgentId,
) -> Option<AgentIdentity> {
    let cipher = cipher();
    postgres::with_tx(pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, Some(&cipher))
            .find(organization_id, agent_id)
            .await
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn an_identity_goes_through_its_lifecycle_with_its_secret_sealed() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let agent_id = agent(&pool, organization_id).await;

    let pending =
        AgentIdentity::pending(organization_id, agent_id, now()).failed("idp down", now());
    store(&pool, &pending).await.unwrap();
    assert_eq!(
        load(&pool, organization_id, agent_id).await,
        Some(pending.clone())
    );

    let active = pending.activated(client("fk-1"), now());
    store(&pool, &active).await.unwrap();
    assert_eq!(
        load(&pool, organization_id, agent_id).await,
        Some(active.clone())
    );

    let (ciphertext,): (Vec<u8>,) =
        sqlx::query_as("SELECT secret_ciphertext FROM agent_identities WHERE agent_id = $1")
            .bind(agent_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        !ciphertext
            .windows(12)
            .any(|window| window == b"agent-secret")
    );

    let disabled = active.disabled(now());
    store(&pool, &disabled).await.unwrap();
    let loaded = load(&pool, organization_id, agent_id).await.unwrap();
    assert_eq!(loaded.status(), IdentityStatus::Disabled);
    assert_eq!(
        loaded.state,
        IdentityState::Disabled {
            client: Some(client("fk-1"))
        }
    );
}

#[tokio::test]
async fn a_secret_moved_to_another_client_does_not_open() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let agent_id = agent(&pool, organization_id).await;
    let active =
        AgentIdentity::pending(organization_id, agent_id, now()).activated(client("fk-1"), now());
    store(&pool, &active).await.unwrap();

    sqlx::query("UPDATE agent_identities SET client_uuid = 'fk-2' WHERE agent_id = $1")
        .bind(agent_id.0)
        .execute(&pool)
        .await
        .unwrap();

    let cipher = cipher();
    let opened = postgres::with_tx(&pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, Some(&cipher))
            .find(organization_id, agent_id)
            .await
    })
    .await;
    assert!(matches!(opened, Err(CoreError::Internal(_))));
}

#[tokio::test]
async fn another_organization_neither_reads_nor_writes_an_identity() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let other = organization(&pool).await;
    let agent_id = agent(&pool, organization_id).await;
    store(
        &pool,
        &AgentIdentity::pending(organization_id, agent_id, now()),
    )
    .await
    .unwrap();

    assert_eq!(load(&pool, other, agent_id).await, None);
    let foreign = AgentIdentity::pending(other, agent_id, now()).activated(client("fk-x"), now());
    assert!(matches!(
        store(&pool, &foreign).await,
        Err(CoreError::NotFound)
    ));
    assert_eq!(
        load(&pool, organization_id, agent_id)
            .await
            .unwrap()
            .status(),
        IdentityStatus::Pending
    );
}

#[tokio::test]
async fn pending_identities_come_least_recently_tried_first() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let older = agent(&pool, organization_id).await;
    let newer = agent(&pool, organization_id).await;
    let active = agent(&pool, organization_id).await;
    let long_ago = now() - Duration::days(3650);
    store(
        &pool,
        &AgentIdentity::pending(organization_id, newer, long_ago + Duration::seconds(1)),
    )
    .await
    .unwrap();
    store(
        &pool,
        &AgentIdentity::pending(organization_id, older, long_ago),
    )
    .await
    .unwrap();
    store(
        &pool,
        &AgentIdentity::pending(organization_id, active, long_ago)
            .activated(client("fk-a"), long_ago),
    )
    .await
    .unwrap();

    let cipher = cipher();
    let pending = postgres::with_tx(&pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, Some(&cipher))
            .pending(10_000)
            .await
    })
    .await
    .unwrap();

    let agents: Vec<AgentId> = pending
        .iter()
        .map(|identity| identity.agent_id)
        .filter(|agent_id| [older, newer, active].contains(agent_id))
        .collect();
    assert_eq!(agents, vec![older, newer]);
}

#[tokio::test]
async fn an_active_identity_needs_the_secrets_key() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let agent_id = agent(&pool, organization_id).await;
    let active =
        AgentIdentity::pending(organization_id, agent_id, now()).activated(client("fk-1"), now());

    let refused = postgres::with_tx(&pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, None)
            .upsert(&active)
            .await
    })
    .await;

    assert!(matches!(refused, Err(CoreError::Internal(_))));
}
