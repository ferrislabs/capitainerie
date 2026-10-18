#![allow(clippy::unwrap_used)]

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{agent::SecretToken, authorization::Subject, execution::GithubAppCredentials},
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

fn subject(organization_id: OrganizationId, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin,
    }
}

fn app_id() -> i64 {
    (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64
}

fn credentials(organization_id: OrganizationId) -> GithubAppCredentials {
    GithubAppCredentials {
        organization_id,
        app_id: app_id(),
        slug: "capitainerie-acme".into(),
        client_id: "Iv1.client".into(),
        client_secret: SecretToken::new("client-secret-value"),
        private_key_pem: SecretToken::new("pem-value"),
        webhook_secret: SecretToken::new("webhook-value"),
        html_url: "https://github.com/apps/capitainerie-acme".into(),
        owner_login: "acme".into(),
        owner_type: "Organization".into(),
        created_by: "someone-else".into(),
        created_at: Utc::now(),
    }
}

fn usecase(pool: &PgPool, byte: u8) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[byte; 32]).unwrap())
}

#[tokio::test]
async fn an_admin_saves_the_app_and_the_system_reads_it_back_decrypted() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let usecase = usecase(&pool, 1);

    let summary = usecase
        .save_github_app(subject(organization, true), credentials(organization))
        .await
        .unwrap();
    let read = usecase.github_app_for(organization).await.unwrap().unwrap();
    let member_view = usecase
        .github_app(subject(organization, false))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(summary, member_view);
    assert_eq!(summary.created_by, "user-1", "the author is the caller");
    assert_eq!(read.client_secret.expose(), "client-secret-value");
    assert_eq!(read.private_key_pem.expose(), "pem-value");
    assert_eq!(read.webhook_secret.expose(), "webhook-value");
    assert_eq!(read.created_at, summary.created_at);
}

#[tokio::test]
async fn a_member_cannot_save_or_delete_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let usecase = usecase(&pool, 1);

    let saved = usecase
        .save_github_app(subject(organization, false), credentials(organization))
        .await;
    let deleted = usecase
        .delete_github_app(subject(organization, false))
        .await;

    assert!(matches!(saved, Err(CoreError::Forbidden { .. })));
    assert!(matches!(deleted, Err(CoreError::Forbidden { .. })));
    assert!(
        usecase
            .github_app_for(organization)
            .await
            .unwrap()
            .is_none()
    );
    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND resource_kind = 'github_app' AND NOT allowed",
    )
    .bind(organization.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denied, 2);
}

#[tokio::test]
async fn a_second_app_is_a_conflict_until_the_first_is_deleted() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let usecase = usecase(&pool, 1);
    let admin = subject(organization, true);
    usecase
        .save_github_app(admin.clone(), credentials(organization))
        .await
        .unwrap();

    let second = usecase
        .save_github_app(admin.clone(), credentials(organization))
        .await;
    let removed = usecase.delete_github_app(admin.clone()).await.unwrap();
    let third = usecase
        .save_github_app(admin.clone(), credentials(organization))
        .await;

    assert!(matches!(second, Err(CoreError::Conflict(code)) if code == "github_app_exists"));
    assert_eq!(removed.slug, "capitainerie-acme");
    assert!(third.is_ok());
    assert!(matches!(
        usecase
            .delete_github_app(subject(OrganizationId::new(), true))
            .await,
        Err(CoreError::NotFound)
    ));
}

#[tokio::test]
async fn sealed_secrets_are_bound_to_their_organization_and_key() {
    let Some(pool) = pool().await else { return };
    let (first, second) = (organization(&pool).await, organization(&pool).await);
    let usecase = usecase(&pool, 1);
    usecase
        .save_github_app(subject(first, true), credentials(first))
        .await
        .unwrap();
    usecase
        .save_github_app(subject(second, true), credentials(second))
        .await
        .unwrap();

    sqlx::query(
        r#"
        UPDATE github_apps AS target
        SET private_key_nonce = source.private_key_nonce,
            private_key_ciphertext = source.private_key_ciphertext
        FROM github_apps AS source
        WHERE target.organization_id = $2 AND source.organization_id = $1
        "#,
    )
    .bind(first.0)
    .bind(second.0)
    .execute(&pool)
    .await
    .unwrap();

    assert!(usecase.github_app_for(first).await.unwrap().is_some());
    assert!(usecase.github_app_for(second).await.is_err());
    assert!(self::usecase(&pool, 2).github_app_for(first).await.is_err());
    assert!(
        CapitainerieUseCase::with_local_policies(pool.clone())
            .github_app_for(first)
            .await
            .is_err()
    );
}
