use capitainerie_common::{CardCommentId, CardId, CoreError, OrganizationId, RunId, TranscriptId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::card::{Author, Card, CardComment, CardTitle, Column, Provenance};

pub(super) struct CardRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub title: String,
    pub description: String,
    pub column: String,
    pub provenance_transcript_id: Option<Uuid>,
    pub provenance_start: Option<i32>,
    pub provenance_end: Option<i32>,
    pub provenance_quote: Option<String>,
    pub created_by_run: Option<Uuid>,
    pub created_by_subject: String,
    pub created_by_actor: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub repository_hint: Option<String>,
}

impl TryFrom<CardRow> for Card {
    type Error = CoreError;

    fn try_from(row: CardRow) -> Result<Self, Self::Error> {
        let provenance = match (
            row.provenance_transcript_id,
            row.provenance_start,
            row.provenance_end,
            row.provenance_quote,
        ) {
            (Some(transcript_id), Some(start), Some(end), Some(quote)) => Some(Provenance {
                transcript_id: TranscriptId(transcript_id),
                start: offset(start)?,
                end: offset(end)?,
                quote,
            }),
            (None, None, None, None) => None,
            _ => {
                return Err(CoreError::Internal(format!(
                    "card {} has a partial provenance",
                    row.id
                )));
            }
        };
        Ok(Self {
            id: CardId(row.id),
            organization_id: OrganizationId(row.organization_id),
            title: CardTitle::try_from(row.title)
                .map_err(|error| CoreError::Internal(format!("stored card title: {error}")))?,
            description: row.description,
            column: Column::try_from(row.column.as_str())
                .map_err(|error| CoreError::Internal(format!("stored card column: {error}")))?,
            provenance,
            created_by_run: row.created_by_run.map(RunId),
            created_by: Author {
                subject: row.created_by_subject,
                actor: row.created_by_actor,
            },
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            repository_hint: row
                .repository_hint
                .map(|value| value.parse())
                .transpose()
                .map_err(|error| CoreError::Internal(format!("stored repository hint: {error}")))?,
        })
    }
}

pub(super) struct CardCommentRow {
    pub id: Uuid,
    pub card_id: Uuid,
    pub organization_id: Uuid,
    pub author_subject: String,
    pub author_actor: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl From<CardCommentRow> for CardComment {
    fn from(row: CardCommentRow) -> Self {
        Self {
            id: CardCommentId(row.id),
            card_id: CardId(row.card_id),
            organization_id: OrganizationId(row.organization_id),
            author: Author {
                subject: row.author_subject,
                actor: row.author_actor,
            },
            body: row.body,
            created_at: row.created_at,
        }
    }
}

fn offset(value: i32) -> Result<u32, CoreError> {
    u32::try_from(value)
        .map_err(|_| CoreError::Internal(format!("negative provenance offset {value}")))
}

pub(super) fn stored_offset(value: u32) -> Result<i32, CoreError> {
    i32::try_from(value)
        .map_err(|_| CoreError::Invalid(format!("provenance offset {value} is too large")))
}
