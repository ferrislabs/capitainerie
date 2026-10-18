#![allow(clippy::unwrap_used)]

use std::str::FromStr;

use capitainerie_common::{OrganizationId, RunId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{execution::TaskGrant, provider::ProviderKind},
};
use chrono::Utc;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use uuid::Uuid;

const ROLE_SQL: &str = include_str!("../../../deploy/postgres/egress-proxy-role.sql");
const ROLE: &str = "capitainerie_egress_proxy";
const PASSWORD: &str = "proxy-role-test";

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

fn random_hash() -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash[..16].copy_from_slice(Uuid::now_v7().as_bytes());
    hash[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    hash
}

async fn as_proxy(url: &str) -> PgPool {
    let owner = PgPool::connect(url).await.unwrap();
    sqlx::raw_sql(ROLE_SQL).execute(&owner).await.unwrap();
    sqlx::query(&format!("ALTER ROLE {ROLE} PASSWORD '{PASSWORD}'"))
        .execute(&owner)
        .await
        .unwrap();
    let options = PgConnectOptions::from_str(url)
        .unwrap()
        .username(ROLE)
        .password(PASSWORD);
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .unwrap()
}

#[tokio::test]
async fn the_proxy_role_reads_what_the_proxy_reads_and_writes_nothing() {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return;
    };
    let pool = as_proxy(&url).await;
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone());
    let organization = OrganizationId(Uuid::now_v7());

    assert_eq!(usecase.find_task_grant(random_hash()).await.unwrap(), None);
    assert!(
        usecase
            .provider_access(organization, ProviderKind::Anthropic)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        usecase
            .github_installation_for(organization)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        usecase
            .github_app_for(organization)
            .await
            .unwrap()
            .is_none()
    );

    let grant = TaskGrant {
        run_id: RunId::new(),
        organization_id: organization,
        repository: "acme/web".parse().unwrap(),
        branch: "capitainerie/x".into(),
        model: "claude-sonnet-5-5".into(),
        expires_at: Utc::now() + chrono::Duration::minutes(1),
        revoked_at: None,
    };
    assert!(
        usecase
            .create_task_grant(grant.run_id, random_hash(), grant)
            .await
            .is_err()
    );
    assert!(usecase.revoke_task_grants(RunId::new()).await.is_err());
    for statement in [
        "INSERT INTO authorization_decisions (id, organization_id, subject, action, resource_kind, allowed) VALUES (gen_random_uuid(), gen_random_uuid(), 's', 'a', 'k', true)",
        "DELETE FROM github_installations",
        "DELETE FROM github_apps",
        "SELECT count(*) FROM runs",
    ] {
        assert!(
            sqlx::query(statement).execute(&pool).await.is_err(),
            "{statement}"
        );
    }
}
