use capitainerie_common::CoreError;
use capitainerie_macros::repository;

use crate::{
    domain::transcript::{Transcript, ports::TranscriptRepository},
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Transcript, backend = Postgres)]
pub struct PgTranscriptRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgTranscriptRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl TranscriptRepository for PgTranscriptRepository<'_> {
    async fn insert(&mut self, transcript: &Transcript) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO transcripts (id, organization_id, submitted_by, title, body, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            transcript.id.0,
            transcript.organization_id.0,
            transcript.submitted_by,
            transcript.title.as_str(),
            transcript.body.as_str(),
            transcript.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}
