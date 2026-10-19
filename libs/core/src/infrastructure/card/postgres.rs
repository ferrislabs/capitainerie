use capitainerie_common::{CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;

use super::model::{CardCommentRow, CardRow, criteria_json, stored_offset};
use crate::{
    domain::{
        card::{
            Card, CardComment,
            attributes::{CardFilter, Estimate, Priority},
            ports::{CardCommentRepository, CardRepository},
        },
        client::links::Links,
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
                created_by_run, created_by_subject, created_by_actor, version, created_at, updated_at,
                client_id, project_id,
                acceptance_criteria, priority, due_date, assignee_subject, labels, estimate
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                    $18, $19, $20, $21, $22, $23)
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
            card.links.client_id().map(|id| id.0),
            card.links.project_id().map(|id| id.0),
            criteria_json(&card.attributes),
            card.attributes.priority.as_str(),
            card.attributes.due_date,
            card.attributes.assignee_subject,
            card.attributes.labels.as_slice(),
            card.attributes.estimate.map(Estimate::as_str),
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
            SELECT c.id, c.organization_id, c.title, c.description, c."column",
                   c.provenance_transcript_id, c.provenance_start, c.provenance_end, c.provenance_quote,
                   c.created_by_run, c.created_by_subject, c.created_by_actor, c.version, c.created_at, c.updated_at,
                   COALESCE(t.repository, p.repository) AS "repository_hint?",
                   c.client_id, cl.name AS "client_name?", c.project_id, p.name AS "project_name?",
                   c.acceptance_criteria, c.priority, c.due_date, c.assignee_subject, c.labels, c.estimate
            FROM cards c
            LEFT JOIN transcripts t ON t.id = c.provenance_transcript_id
            LEFT JOIN clients cl ON cl.id = c.client_id
            LEFT JOIN projects p ON p.id = c.project_id
            WHERE c.organization_id = $1 AND c.id = $2
            FOR UPDATE OF c
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

    async fn get(
        &mut self,
        organization_id: OrganizationId,
        id: CardId,
    ) -> Result<Option<Card>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            CardRow,
            r#"
            SELECT c.id, c.organization_id, c.title, c.description, c."column",
                   c.provenance_transcript_id, c.provenance_start, c.provenance_end, c.provenance_quote,
                   c.created_by_run, c.created_by_subject, c.created_by_actor, c.version, c.created_at, c.updated_at,
                   COALESCE(t.repository, p.repository) AS "repository_hint?",
                   c.client_id, cl.name AS "client_name?", c.project_id, p.name AS "project_name?",
                   c.acceptance_criteria, c.priority, c.due_date, c.assignee_subject, c.labels, c.estimate
            FROM cards c
            LEFT JOIN transcripts t ON t.id = c.provenance_transcript_id
            LEFT JOIN clients cl ON cl.id = c.client_id
            LEFT JOIN projects p ON p.id = c.project_id
            WHERE c.organization_id = $1 AND c.id = $2
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
            SET title = $3, description = $4, "column" = $5, version = $6, updated_at = $7,
                client_id = $8, project_id = $9,
                acceptance_criteria = $10, priority = $11, due_date = $12, assignee_subject = $13,
                labels = $14, estimate = $15
            WHERE organization_id = $1 AND id = $2 AND version = $6 - 1
            "#,
            card.organization_id.0,
            card.id.0,
            card.title.as_str(),
            card.description,
            card.column.as_str(),
            card.version,
            card.updated_at,
            card.links.client_id().map(|id| id.0),
            card.links.project_id().map(|id| id.0),
            criteria_json(&card.attributes),
            card.attributes.priority.as_str(),
            card.attributes.due_date,
            card.attributes.assignee_subject,
            card.attributes.labels.as_slice(),
            card.attributes.estimate.map(Estimate::as_str),
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

    async fn link_run_cards(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
        span: Option<(u32, u32)>,
        links: &Links,
    ) -> Result<Vec<(CardId, i32)>, CoreError> {
        let (start, end) = match span {
            Some((start, end)) => (Some(stored_offset(start)?), Some(stored_offset(end)?)),
            None => (None, None),
        };
        let mut tx = self.tx.lock().await;
        let linked = sqlx::query!(
            r#"
            UPDATE cards
            SET client_id = $5, project_id = $6, version = version + 1, updated_at = now()
            WHERE organization_id = $1 AND created_by_run = $2
              AND client_id IS NULL AND project_id IS NULL
              AND ($3::int IS NULL OR (provenance_start = $3 AND provenance_end = $4))
            RETURNING id, version
            "#,
            organization_id.0,
            run_id.0,
            start,
            end,
            links.client_id().map(|id| id.0),
            links.project_id().map(|id| id.0),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(linked
            .into_iter()
            .map(|row| (CardId(row.id), row.version))
            .collect())
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &CardFilter,
    ) -> Result<Vec<Card>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            CardRow,
            r#"
            SELECT c.id, c.organization_id, c.title, c.description, c."column",
                   c.provenance_transcript_id, c.provenance_start, c.provenance_end, c.provenance_quote,
                   c.created_by_run, c.created_by_subject, c.created_by_actor, c.version, c.created_at, c.updated_at,
                   COALESCE(t.repository, p.repository) AS "repository_hint?",
                   c.client_id, cl.name AS "client_name?", c.project_id, p.name AS "project_name?",
                   c.acceptance_criteria, c.priority, c.due_date, c.assignee_subject, c.labels, c.estimate
            FROM cards c
            LEFT JOIN transcripts t ON t.id = c.provenance_transcript_id
            LEFT JOIN clients cl ON cl.id = c.client_id
            LEFT JOIN projects p ON p.id = c.project_id
            WHERE c.organization_id = $1
              AND ($2::uuid IS NULL OR c.client_id = $2)
              AND ($3::uuid IS NULL OR c.project_id = $3)
              AND ($4::text IS NULL OR c.priority = $4)
              AND ($5::text IS NULL OR c.assignee_subject = $5)
              AND ($6::text IS NULL
                   OR EXISTS (SELECT 1 FROM unnest(c.labels) AS label WHERE lower(label) = lower($6)))
            ORDER BY c.updated_at DESC, c.id DESC
            LIMIT 500
            "#,
            organization_id.0,
            filter.client_id.map(|id| id.0),
            filter.project_id.map(|id| id.0),
            filter.priority.map(Priority::as_str),
            filter.assignee,
            filter.label,
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
