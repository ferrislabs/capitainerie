pub mod ports;
pub mod service;
pub mod turn;

use capitainerie_common::{ConversationId, ConversationMessageId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::provider::ModelRef;

/// The client id the Captain acts through; the policy decision point knows it
/// as the actor of every tool call.
pub const CAPTAIN_AGENT: &str = "capitainerie-agent-captain";

pub const MAX_TOOL_CALLS_PER_TURN: usize = 20;
pub const MAX_OUTPUT_TOKENS_PER_TURN: u32 = 16_000;
pub const MAX_TOKENS_PER_CONVERSATION: u64 = 400_000;

pub const TITLE_FROM_MESSAGE_CHARS: usize = 60;
pub const TITLE_MAX_CHARS: usize = 120;
pub const MESSAGE_MAX_CHARS: usize = 20_000;

/// A turn whose lease is older than this is considered dead (the process that
/// held it crashed), so the conversation does not stay locked forever.
pub const TURN_LEASE_SECS: i64 = 600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub id: ConversationId,
    pub organization_id: OrganizationId,
    pub owner_subject: String,
    pub title: String,
    pub model: Option<ModelRef>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Complete,
    Interrupted,
    Failed,
}

/// One tool call as kept in the conversation: what was asked (summarised),
/// whether it worked and how long it took.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRecord {
    pub name: String,
    pub arguments: String,
    pub succeeded: bool,
    pub latency_ms: u64,
    pub summary: String,
}

/// A card the Captain proposed during a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedCard {
    pub card_id: Uuid,
    pub title: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageContent {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolRecord>,
    #[serde(default)]
    pub cards: Vec<ProposedCard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: ConversationMessageId,
    pub conversation_id: ConversationId,
    pub seq: i32,
    pub role: MessageRole,
    pub content: MessageContent,
    pub status: MessageStatus,
    pub created_at: DateTime<Utc>,
}

impl MessageRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

impl TryFrom<&str> for MessageRole {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        [Self::User, Self::Assistant, Self::Tool]
            .into_iter()
            .find(|role| role.as_str() == value)
            .ok_or_else(|| CoreError::Internal(format!("unknown message role {value}")))
    }
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }
}

impl TryFrom<&str> for MessageStatus {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        [Self::Complete, Self::Interrupted, Self::Failed]
            .into_iter()
            .find(|status| status.as_str() == value)
            .ok_or_else(|| CoreError::Internal(format!("unknown message status {value}")))
    }
}

impl Conversation {
    pub fn tokens(&self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }

    pub fn is_full(&self) -> bool {
        self.tokens() >= MAX_TOKENS_PER_CONVERSATION
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

/// The title of a conversation: the first line of its first message, cut at 60
/// characters.
pub fn title_of(message: &str) -> String {
    let line = message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    line.chars().take(TITLE_FROM_MESSAGE_CHARS).collect()
}

pub fn valid_message(message: String) -> Result<String, CoreError> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Invalid("the message is empty".into()));
    }
    if trimmed.chars().count() > MESSAGE_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "the message exceeds {MESSAGE_MAX_CHARS} characters"
        )));
    }
    Ok(trimmed.to_owned())
}

pub fn valid_title(title: String) -> Result<String, CoreError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Invalid("the title is empty".into()));
    }
    if trimmed.chars().count() > TITLE_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "the title exceeds {TITLE_MAX_CHARS} characters"
        )));
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conversation(input: u64, output: u64) -> Conversation {
        let now = Utc::now();
        Conversation {
            id: ConversationId::new(),
            organization_id: OrganizationId::new(),
            owner_subject: "user-1".into(),
            title: "t".into(),
            model: None,
            input_tokens: input,
            output_tokens: output,
            cost_micros: 0,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }

    #[test]
    fn the_title_is_the_first_line_cut_at_60_characters() {
        assert_eq!(
            title_of("  \n  Ajoute un README  \nsuite"),
            "Ajoute un README"
        );
        let long = "é".repeat(80);
        assert_eq!(title_of(&long).chars().count(), TITLE_FROM_MESSAGE_CHARS);
        assert_eq!(title_of(" \n "), "");
    }

    #[test]
    fn a_conversation_is_full_at_400_000_tokens_in_and_out() {
        assert!(!conversation(300_000, 99_999).is_full());
        assert!(conversation(300_000, 100_000).is_full());
        assert!(conversation(u64::MAX, 1).is_full());
    }

    #[test]
    fn messages_and_titles_are_trimmed_and_bounded() {
        assert_eq!(valid_message("  salut \n".into()).unwrap(), "salut");
        assert!(valid_message("  ".into()).is_err());
        assert!(valid_message("x".repeat(MESSAGE_MAX_CHARS + 1)).is_err());
        assert!(valid_message("x".repeat(MESSAGE_MAX_CHARS)).is_ok());
        assert!(valid_title(" ".into()).is_err());
        assert!(valid_title("x".repeat(TITLE_MAX_CHARS + 1)).is_err());
        assert_eq!(valid_title(" Mon fil ".into()).unwrap(), "Mon fil");
    }

    #[test]
    fn roles_and_statuses_round_trip_through_text() {
        for role in [MessageRole::User, MessageRole::Assistant, MessageRole::Tool] {
            assert_eq!(MessageRole::try_from(role.as_str()).unwrap(), role);
        }
        for status in [
            MessageStatus::Complete,
            MessageStatus::Interrupted,
            MessageStatus::Failed,
        ] {
            assert_eq!(MessageStatus::try_from(status.as_str()).unwrap(), status);
        }
        assert!(MessageRole::try_from("system").is_err());
        assert!(MessageStatus::try_from("pending").is_err());
    }

    #[test]
    fn a_stored_content_without_tool_calls_still_loads() {
        let content: MessageContent = serde_json::from_str(r#"{"text":"salut"}"#).unwrap();
        assert_eq!(content.text, "salut");
        assert!(content.tool_calls.is_empty() && content.cards.is_empty());
    }
}
