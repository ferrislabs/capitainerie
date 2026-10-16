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
