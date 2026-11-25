#![allow(clippy::unwrap_used)]

//! The sweep reads every organization's identities, so each test runs on a
//! database of its own: the shared one holds other tests' identities.

use std::sync::Mutex;

use capitainerie_common::{AgentId, CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, IdentitySweep,
    domain::{
        agent::SecretToken,
        agent_identity::{
            AgentClientId, AgentIdentity, IdentityState, ProvisionedClient,
            ports::{AgentIdentityProvisioner, AgentIdentityRepository},
        },
    },
    infrastructure::{
        agent_identity::postgres::PgAgentIdentityRepository, crypto::SecretCipher, postgres,
    },
};
use chrono::{Duration, Utc};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

const ISSUER: &str = "http://idp.test/realms/acme";

struct Database {
    pool: PgPool,
    admin_url: String,
    name: String,
}

impl Database {
    async fn create() -> Option<Self> {
        let admin_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
                panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
            }
            Err(_) => {
                eprintln!("skipped: DATABASE_URL is unset");
                return None;
            }
        };
        let name = format!("identity_sweep_{}", Uuid::now_v7().simple());
        let mut admin = PgConnection::connect(&admin_url).await.unwrap();
        admin
            .execute(format!("CREATE DATABASE {name}").as_str())
            .await
            .unwrap();
        let (base, _) = admin_url.rsplit_once('/').unwrap();
        let pool = postgres::connect(&format!("{base}/{name}"), 2)
            .await
            .unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        Some(Self {
            pool,
            admin_url,
            name,
        })
    }

    async fn drop(self) {
        self.pool.close().await;
        let mut admin = PgConnection::connect(&self.admin_url).await.unwrap();
        admin
            .execute(format!("DROP DATABASE {} WITH (FORCE)", self.name).as_str())
            .await
            .unwrap();
    }
}

#[derive(Default)]
struct FakeFerrisKey {
    refuse: bool,
    provisioned: Mutex<Vec<String>>,
    toggled: Mutex<Vec<(String, bool)>>,
}

impl AgentIdentityProvisioner for FakeFerrisKey {
    async fn provision(
        &self,
        issuer: &str,
        client_id: &AgentClientId,
        _name: &str,
    ) -> Result<ProvisionedClient, CoreError> {
        assert_eq!(issuer, ISSUER);
        if self.refuse {
            return Err(CoreError::Internal("ferriskey is down".into()));
        }
        self.provisioned
            .lock()
            .unwrap()
            .push(client_id.as_str().to_owned());
        Ok(ProvisionedClient {
            client_uuid: format!("fk-{}", client_id.as_str()),
            secret: SecretToken::new("agent-secret"),
        })
    }

    async fn set_enabled(
        &self,
        _issuer: &str,
        client_uuid: &str,
        enabled: bool,
    ) -> Result<(), CoreError> {
        self.toggled
            .lock()
            .unwrap()
            .push((client_uuid.to_owned(), enabled));
        Ok(())
    }
}

fn cipher() -> SecretCipher {
    SecretCipher::new(&[9; 32]).unwrap()
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone()).with_secrets_cipher(cipher())
}

async fn organization(pool: &PgPool) -> OrganizationId {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(format!("test-{}", id.simple()))
    .bind(ISSUER)
    .execute(pool)
    .await
    .unwrap();
    OrganizationId(id)
}

async fn agent(pool: &PgPool, organization_id: OrganizationId, archived: bool) -> AgentId {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO agents (id, organization_id, slug, name, archived, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, 'Bernard', $4, 'test', now(), now())",
    )
    .bind(id)
    .bind(organization_id.0)
    .bind(format!("a-{}", id.simple()))
    .bind(archived)
    .execute(pool)
    .await
    .unwrap();
    AgentId(id)
}

async fn load(pool: &PgPool, organization_id: OrganizationId, agent_id: AgentId) -> AgentIdentity {
    let cipher = cipher();
    postgres::with_tx(pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, Some(&cipher))
            .find(organization_id, agent_id)
            .await
    })
    .await
    .unwrap()
    .unwrap()
}

async fn store(pool: &PgPool, identity: &AgentIdentity) {
    let cipher = cipher();
    postgres::with_tx(pool, async |tx| {
        PgAgentIdentityRepository::new(&tx, Some(&cipher))
            .upsert(identity)
            .await
    })
    .await
    .unwrap();
}

fn retry_before() -> chrono::DateTime<Utc> {
    Utc::now() - Duration::seconds(60)
}

#[tokio::test]
async fn a_sweep_adopts_agents_provisions_them_and_follows_archives() {
    let Some(database) = Database::create().await else {
        return;
    };
    let pool = &database.pool;
    let organization_id = organization(pool).await;
    let bernard = agent(pool, organization_id, false).await;
    let archived = agent(pool, organization_id, true).await;
    let leaving = agent(pool, organization_id, true).await;
    store(
        pool,
        &AgentIdentity::pending(organization_id, leaving, Utc::now())
            .activated(
                ProvisionedClient {
                    client_uuid: "fk-leaving".into(),
                    secret: SecretToken::new("agent-secret"),
                },
                Utc::now(),
            )
            .disabled(Utc::now()),
    )
    .await;
    let ferriskey = FakeFerrisKey::default();
    let usecase = usecase(pool);

    let sweep = usecase
        .sweep_agent_identities(&ferriskey, retry_before(), 20)
        .await
        .unwrap();

    assert_eq!(
        sweep,
        IdentitySweep {
            adopted: 2,
            picked: 2,
            activated: 1,
            synced: 1,
            failed: 0,
        }
    );
    assert_eq!(
        *ferriskey.provisioned.lock().unwrap(),
        vec![AgentClientId::of(bernard).as_str().to_owned()]
    );
    assert_eq!(
        *ferriskey.toggled.lock().unwrap(),
        vec![("fk-leaving".to_owned(), false)]
    );
    assert!(
        load(pool, organization_id, bernard)
            .await
            .credential()
            .is_some()
    );
    assert_eq!(
        load(pool, organization_id, archived).await.state,
        IdentityState::Disabled { client: None }
    );
    assert!(load(pool, organization_id, leaving).await.synced);

    let again = usecase
        .sweep_agent_identities(&ferriskey, retry_before(), 20)
        .await
        .unwrap();
    assert_eq!(again, IdentitySweep::default());

    database.drop().await;
}

#[tokio::test]
async fn a_refused_provisioning_waits_before_the_next_attempt() {
    let Some(database) = Database::create().await else {
        return;
    };
    let pool = &database.pool;
    let organization_id = organization(pool).await;
    let bernard = agent(pool, organization_id, false).await;
    let down = FakeFerrisKey {
        refuse: true,
        ..FakeFerrisKey::default()
    };
    let usecase = usecase(pool);

    let sweep = usecase
        .sweep_agent_identities(&down, retry_before(), 20)
        .await
        .unwrap();
    assert_eq!(sweep.failed, 1);
    let IdentityState::Pending {
        failure: Some(failure),
    } = load(pool, organization_id, bernard).await.state
    else {
        panic!("still pending with the reason");
    };
    assert!(failure.contains("ferriskey is down"));

    let up = FakeFerrisKey::default();
    let too_soon = usecase
        .sweep_agent_identities(&up, retry_before(), 20)
        .await
        .unwrap();
    assert_eq!(too_soon.picked, 0);

    let later = usecase
        .sweep_agent_identities(&up, Utc::now() + Duration::seconds(1), 20)
        .await
        .unwrap();
    assert_eq!(later.activated, 1);
    assert!(
        load(pool, organization_id, bernard)
            .await
            .credential()
            .is_some()
    );

    database.drop().await;
}
