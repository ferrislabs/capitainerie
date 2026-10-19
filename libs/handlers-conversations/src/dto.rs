use capitainerie_common::ConversationId;
use capitainerie_core::domain::conversation::{
    Conversation, Message, MessageRole, MessageStatus, ProposedCard, ToolRecord,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageRoleDto {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatusDto {
    Complete,
    Interrupted,
    Failed,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatToolCallResponse {
    pub name: String,
    /// The arguments, summarised.
    pub arguments: String,
    pub succeeded: bool,
    pub latency_ms: u64,
    /// What the member reads: « 3 cartes trouvées », the title of a proposed card, the error.
    pub summary: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProposedCardResponse {
    pub card_id: Uuid,
    pub title: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub id: Uuid,
    pub seq: i32,
    pub role: MessageRoleDto,
    pub text: String,
    pub tool_calls: Vec<ChatToolCallResponse>,
    /// The cards the Captain proposed in this message; their current state
    /// comes from the card endpoints.
    pub cards: Vec<ProposedCardResponse>,
    pub status: MessageStatusDto,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationSummaryResponse {
    pub id: ConversationId,
    pub title: String,
    /// The model that answered last.
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
    /// The conversation reached its token limit: open a new one.
    pub full: bool,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationSummaryResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationResponse {
    pub conversation: ConversationSummaryResponse,
    pub messages: Vec<MessageResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedConversationResponse {
    pub id: ConversationId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateConversationRequest {
    /// The first message of the member; it gives the conversation its title.
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    /// The member's message. Absent, the last message that has no answer yet
    /// is answered: that is how the first turn of a new conversation starts.
    pub message: Option<String>,
}

impl From<MessageRole> for MessageRoleDto {
    fn from(role: MessageRole) -> Self {
        match role {
            MessageRole::User => Self::User,
            MessageRole::Assistant => Self::Assistant,
            MessageRole::Tool => Self::Tool,
        }
    }
}

impl From<MessageStatus> for MessageStatusDto {
    fn from(status: MessageStatus) -> Self {
        match status {
            MessageStatus::Complete => Self::Complete,
            MessageStatus::Interrupted => Self::Interrupted,
            MessageStatus::Failed => Self::Failed,
        }
    }
}

impl From<ToolRecord> for ChatToolCallResponse {
    fn from(record: ToolRecord) -> Self {
        Self {
            name: record.name,
            arguments: record.arguments,
            succeeded: record.succeeded,
            latency_ms: record.latency_ms,
            summary: record.summary,
        }
    }
}

impl From<ProposedCard> for ProposedCardResponse {
    fn from(card: ProposedCard) -> Self {
        Self {
            card_id: card.card_id,
            title: card.title,
        }
    }
}

impl From<Message> for MessageResponse {
    fn from(message: Message) -> Self {
        Self {
            id: message.id.0,
            seq: message.seq,
            role: message.role.into(),
            text: message.content.text,
            tool_calls: message
                .content
                .tool_calls
                .into_iter()
                .map(Into::into)
                .collect(),
            cards: message.content.cards.into_iter().map(Into::into).collect(),
            status: message.status.into(),
            created_at: message.created_at,
        }
    }
}

impl From<&Conversation> for ConversationSummaryResponse {
    fn from(conversation: &Conversation) -> Self {
        Self {
            id: conversation.id,
            title: conversation.title.clone(),
            model: conversation.model.as_ref().map(|model| model.model.clone()),
            input_tokens: conversation.input_tokens,
            output_tokens: conversation.output_tokens,
            cost_micros: conversation.cost_micros,
            full: conversation.is_full(),
            archived: conversation.is_archived(),
            created_at: conversation.created_at,
            updated_at: conversation.updated_at,
        }
    }
}
