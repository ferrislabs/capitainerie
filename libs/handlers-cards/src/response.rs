use capitainerie_common::{CardCommentId, CardId, RunId, TranscriptId};
use capitainerie_core::domain::card::{Card, CardComment, Column, detail::CardDetail};
use capitainerie_handlers_runs::response::{
    ClientRefResponse, ProjectRefResponse, RunResponse, link_refs,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColumnDto {
    Proposed,
    Todo,
    InProgress,
    Done,
    Rejected,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProvenanceResponse {
    pub transcript_id: TranscriptId,
    pub start: u32,
    pub end: u32,
    pub quote: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CardResponse {
    pub id: CardId,
    pub title: String,
    pub description: String,
    pub column: ColumnDto,
    pub provenance: Option<ProvenanceResponse>,
    pub created_by_run: Option<RunId>,
    pub created_by_subject: String,
    /// Client id of the agent that proposed the card, when one did.
    pub created_by_agent: Option<String>,
    pub repository_hint: Option<String>,
    pub client: Option<ClientRefResponse>,
    pub project: Option<ProjectRefResponse>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Column> for ColumnDto {
    fn from(column: Column) -> Self {
        match column {
            Column::Proposed => Self::Proposed,
            Column::Todo => Self::Todo,
            Column::InProgress => Self::InProgress,
            Column::Done => Self::Done,
            Column::Rejected => Self::Rejected,
        }
    }
}

impl From<ColumnDto> for Column {
    fn from(column: ColumnDto) -> Self {
        match column {
            ColumnDto::Proposed => Self::Proposed,
            ColumnDto::Todo => Self::Todo,
            ColumnDto::InProgress => Self::InProgress,
            ColumnDto::Done => Self::Done,
            ColumnDto::Rejected => Self::Rejected,
        }
    }
}

impl From<Card> for CardResponse {
    fn from(card: Card) -> Self {
        let (client, project) = link_refs(&card.links);
        Self {
            id: card.id,
            title: card.title.as_str().to_owned(),
            description: card.description,
            column: card.column.into(),
            provenance: card.provenance.map(|p| ProvenanceResponse {
                transcript_id: p.transcript_id,
                start: p.start,
                end: p.end,
                quote: p.quote,
            }),
            created_by_run: card.created_by_run,
            created_by_subject: card.created_by.subject,
            created_by_agent: card.created_by.actor,
            repository_hint: card.repository_hint.map(|hint| hint.to_string()),
            client,
            project,
            version: card.version,
            created_at: card.created_at,
            updated_at: card.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommentResponse {
    pub id: CardCommentId,
    pub body: String,
    pub author_subject: String,
    /// Client id of the agent that wrote the comment, when one did.
    pub author_actor: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CardDetailResponse {
    pub card: CardResponse,
    /// Oldest first.
    pub comments: Vec<CommentResponse>,
    /// The run that proposed the card, then its executions, newest first.
    pub runs: Vec<RunResponse>,
}

impl From<CardComment> for CommentResponse {
    fn from(comment: CardComment) -> Self {
        Self {
            id: comment.id,
            body: comment.body,
            author_subject: comment.author.subject,
            author_actor: comment.author.actor,
            created_at: comment.created_at,
        }
    }
}

impl From<CardDetail> for CardDetailResponse {
    fn from(detail: CardDetail) -> Self {
        Self {
            card: detail.card.into(),
            comments: detail.comments.into_iter().map(Into::into).collect(),
            runs: detail.runs.into_iter().map(Into::into).collect(),
        }
    }
}
