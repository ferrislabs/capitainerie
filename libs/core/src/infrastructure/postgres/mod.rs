use capitainerie_common::CoreError;
use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn connect(url: &str, max_connections: u32) -> Result<PgPool, CoreError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(url)
        .await
        .map_err(map_sqlx_error)
}

pub async fn ping(pool: &PgPool) -> Result<(), CoreError> {
    sqlx::query_scalar!(r#"SELECT 1 AS "one!""#)
        .fetch_one(pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
}

pub fn map_sqlx_error(error: sqlx::Error) -> CoreError {
    match error {
        sqlx::Error::RowNotFound => CoreError::NotFound,
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            CoreError::Conflict(db.message().to_owned())
        }
        other => CoreError::Database(other.to_string()),
    }
}
