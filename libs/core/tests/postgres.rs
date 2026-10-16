use capitainerie_core::{CapitainerieUseCase, infrastructure::postgres};

/// Skips when `DATABASE_URL` is unset; `REQUIRE_DATABASE_URL=1` turns the skip
/// into a failure so CI cannot pass without a database.
fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

#[tokio::test]
async fn readiness_succeeds_against_a_live_database() {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return;
    };
    let pool = postgres::connect(&url, 1).await.unwrap();
    CapitainerieUseCase::new(pool).readiness().await.unwrap();
}

#[tokio::test]
async fn organization_by_issuer_finds_only_known_realms() {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return;
    };
    let pool = postgres::connect(&url, 1).await.unwrap();
    let id = uuid::Uuid::now_v7();
    let issuer = format!("http://idp.test/realms/{id}");
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(format!("test-{}", id.simple()))
    .bind(&issuer)
    .execute(&pool)
    .await
    .unwrap();
    let usecase = CapitainerieUseCase::new(pool.clone());

    let found = usecase.organization_by_issuer(&issuer).await;
    let unknown = usecase
        .organization_by_issuer("http://idp.test/realms/unknown")
        .await;
    sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(found.unwrap().id.0, id);
    assert!(matches!(
        unknown,
        Err(capitainerie_common::CoreError::NotFound)
    ));
}
