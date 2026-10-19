use capitainerie_common::{CardId, CoreError, OrganizationId};
use capitainerie_macros::repository;

use super::model::{CardCommentRow, CardRow, stored_offset};
use crate::{
    domain::card::{
        Card, CardComment,
        ports::{CardCommentRepository, CardRepository},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Card, backend = Postgres)]
pub struct PgCardRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgCardRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl CardRepository for PgCardRepository<'_> {
    async fn insert(&mut self, card: &Card) -> Result<(), CoreError> {
        let provenance = card.provenance.as_ref();
        let start = provenance.map(|p| stored_offset(p.start)).transpose()?;
        let end = provenance.map(|p| stored_offset(p.end)).transpose()?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO cards (
                id, organization_id, title, description, "column",
                provenance_transcript_id, provenance_start, provenance_end, provenance_quote,
                created_by_run, created_by_subject, created_by_actor, version, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            card.id.0,
            card.organization_id.0,
            card.title.as_str(),
            card.description,
            card.column.as_str(),
            provenance.map(|p| p.transcript_id.0),
            start,
            end,
            provenance.map(|p| p.quote.as_str()),
            card.created_by_run.map(|id| id.0),
            card.created_by.subject,
            card.created_by.actor,
            card.version,
            card.created_at,
            card.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: CardId,
    ) -> Result<Option<Card>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            CardRow,
            r#"
            SELECT id, organization_id, title, description, "column",
                   provenance_transcript_id, provenance_start, provenance_end, provenance_quote,
                   created_by_run, created_by_subject, created_by_actor, version, created_at, updated_at
            FROM cards
            WHERE organization_id = $1 AND id = $2
            FOR UPDATE
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Card::try_from)
        .transpose()
    }

    async fn update(&mut self, card: &Card) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE cards
            SET title = $3, description = $4, "column" = $5, version = $6, updated_at = $7
            WHERE organization_id = $1 AND id = $2 AND version = $6 - 1
            "#,
            card.organization_id.0,
            card.id.0,
            card.title.as_str(),
            card.description,
            card.column.as_str(),
            card.version,
            card.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::Conflict(format!(
                "card {} changed meanwhile: read it again",
                card.id
            )));
        }
        Ok(())
    }

    async fn list(&mut self, organization_id: OrganizationId) -> Result<Vec<Card>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            CardRow,
            r#"
            SELECT id, organization_id, title, description, "column",
                   provenance_transcript_id, provenance_start, provenance_end, provenance_quote,
                   created_by_run, created_by_subject, created_by_actor, version, created_at, updated_at
            FROM cards
            WHERE organization_id = $1
            ORDER BY updated_at DESC, id DESC
            LIMIT 500
            "#,
            organization_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Card::try_from)
        .collect()
    }
}

#[repository(domain = CardComment, backend = Postgres)]
pub struct PgCardCommentRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgCardCommentRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl CardCommentRepository for PgCardCommentRepository<'_> {
    async fn insert(&mut self, comment: &CardComment) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO card_comments (id, card_id, organization_id, author_subject, author_actor, body, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            comment.id.0,
            comment.card_id.0,
            comment.organization_id.0,
            comment.author.subject,
            comment.author.actor,
            comment.body,
            comment.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
    async fn list(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> Result<Vec<CardComment>, CoreError> {
        let mut tx = self.tx.lock().await;
        Ok(sqlx::query_as!(
            CardCommentRow,
            r#"
            SELECT id, card_id, organization_id, author_subject, author_actor, body, created_at
            FROM card_comments
            WHERE organization_id = $1 AND card_id = $2
            ORDER BY created_at, id
            "#,
            organization_id.0,
            card_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(CardComment::from)
        .collect())
    }
}
