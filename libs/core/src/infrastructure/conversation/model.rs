use capitainerie_common::{ConversationId, ConversationMessageId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};
use sqlx::types::Json;
use uuid::Uuid;

use crate::domain::{
    conversation::{Conversation, Message, MessageContent, MessageRole, MessageStatus},
    provider::ModelRef,
};

pub(super) struct ConversationRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub owner_subject: String,
    pub title: String,
    pub model: Option<Json<ModelRef>>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<ConversationRow> for Conversation {
    type Error = CoreError;

    fn try_from(row: ConversationRow) -> Result<Self, Self::Error> {
        let counter = |value: i64, what: &str| {
            u64::try_from(value).map_err(|_| {
                CoreError::Internal(format!("negative {what} on conversation {}", row.id))
            })
        };
        Ok(Self {
            id: ConversationId(row.id),
            organization_id: OrganizationId(row.organization_id),
            owner_subject: row.owner_subject.clone(),
            title: row.title.clone(),
            model: row.model.as_ref().map(|model| model.0.clone()),
            input_tokens: counter(row.input_tokens, "input tokens")?,
            output_tokens: counter(row.output_tokens, "output tokens")?,
            cost_micros: row.cost_micros,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}

pub(super) struct MessageRow {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub seq: i32,
    pub role: String,
    pub content: Json<MessageContent>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<MessageRow> for Message {
    type Error = CoreError;

    fn try_from(row: MessageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ConversationMessageId(row.id),
            conversation_id: ConversationId(row.conversation_id),
            seq: row.seq,
            role: MessageRole::try_from(row.role.as_str())?,
            content: row.content.0,
            status: MessageStatus::try_from(row.status.as_str())?,
            created_at: row.created_at,
        })
    }
}

pub(super) fn stored_counter(value: u64, what: &str) -> Result<i64, CoreError> {
    i64::try_from(value).map_err(|_| CoreError::Invalid(format!("{what} is too large")))
}
