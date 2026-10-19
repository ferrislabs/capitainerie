use capitainerie_common::{ConversationId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use super::{Conversation, Message, MessageContent, MessageRole, MessageStatus};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait ConversationRepository: Send {
    fn insert(
        &mut self,
        conversation: &Conversation,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Scoped to the owner: another member's conversation is not found.
    fn find(
        &mut self,
        organization_id: OrganizationId,
        owner_subject: &str,
        id: ConversationId,
    ) -> impl Future<Output = Result<Option<Conversation>, CoreError>> + Send;

    /// The owner's conversations, not archived, the most recent first.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        owner_subject: &str,
    ) -> impl Future<Output = Result<Vec<Conversation>, CoreError>> + Send;

    fn update(
        &mut self,
        conversation: &Conversation,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Takes the conversation's single turn slot. `false` when another turn
    /// holds it and its lease has not expired.
    fn begin_turn(
        &mut self,
        id: ConversationId,
        now: DateTime<Utc>,
        lease_secs: i64,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// Frees the turn slot only if it is still the one `leased_at` took: a
    /// turn whose lease expired and was taken over must not free the new one.
    fn end_turn(
        &mut self,
        id: ConversationId,
        leased_at: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait MessageRepository: Send {
    /// Stores the message with the next sequence number of its conversation.
    fn append(
        &mut self,
        conversation_id: ConversationId,
        role: MessageRole,
        content: &MessageContent,
        status: MessageStatus,
    ) -> impl Future<Output = Result<Message, CoreError>> + Send;

    /// Oldest first.
    fn list(
        &mut self,
        conversation_id: ConversationId,
    ) -> impl Future<Output = Result<Vec<Message>, CoreError>> + Send;
}
