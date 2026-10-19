use capitainerie_common::{ConversationId, CoreError};
use chrono::Utc;

use super::{
    Conversation, Message, MessageContent, MessageRole, MessageStatus, TURN_LEASE_SECS,
    ports::{ConversationRepository, MessageRepository},
    title_of,
    turn::{ChatMessage, ChatRole, TurnError},
    valid_message, valid_title,
};
use crate::domain::{
    agent::Usage,
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    provider::ModelRef,
};

#[derive(Debug, Clone, Default)]
pub struct UpdateConversation {
    pub title: Option<String>,
    pub archived: Option<bool>,
}

/// What a turn leaves behind: the assistant message and what it cost.
#[derive(Debug, Clone)]
pub struct TurnOutcome {
    pub content: MessageContent,
    pub status: MessageStatus,
    pub usage: Usage,
    pub cost_micros: i64,
    pub model: ModelRef,
}

#[derive(Debug, Clone)]
pub struct StartedTurn {
    pub conversation: Conversation,
    pub history: Vec<ChatMessage>,
}

pub struct ConversationService<'a, C, M, A> {
    conversations: C,
    messages: M,
    authorizer: &'a A,
}

impl<'a, C, M, A> ConversationService<'a, C, M, A>
where
    C: ConversationRepository,
    M: MessageRepository,
    A: Authorizer,
{
    pub fn new(conversations: C, messages: M, authorizer: &'a A) -> Self {
        Self {
            conversations,
            messages,
            authorizer,
        }
    }

    pub async fn create(
        mut self,
        subject: Subject,
        message: String,
    ) -> Result<Conversation, CoreError> {
        self.require(&subject, Action::ConversationWrite, None)
            .await?;
        let message = valid_message(message)?;
        let now = Utc::now();
        let conversation = Conversation {
            id: ConversationId::new(),
            organization_id: subject.organization_id,
            owner_subject: subject.id,
            title: title_of(&message),
            model: None,
            input_tokens: 0,
            output_tokens: 0,
            cost_micros: 0,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        self.conversations.insert(&conversation).await?;
        self.messages
            .append(
                conversation.id,
                MessageRole::User,
                &MessageContent {
                    text: message,
                    ..MessageContent::default()
                },
                MessageStatus::Complete,
            )
            .await?;
        Ok(conversation)
    }

    pub async fn list(mut self, subject: Subject) -> Result<Vec<Conversation>, CoreError> {
        self.require(&subject, Action::ConversationRead, None)
            .await?;
        self.conversations
            .list(subject.organization_id, &subject.id)
            .await
    }

    pub async fn get(
        mut self,
        subject: Subject,
        id: ConversationId,
    ) -> Result<(Conversation, Vec<Message>), CoreError> {
        self.require(&subject, Action::ConversationRead, Some(id))
            .await?;
        let conversation = self.owned(&subject, id).await?;
        let messages = self.messages.list(id).await?;
        Ok((conversation, messages))
    }

    /// Checks the caller may talk in this conversation, before anything else
    /// is looked at: a conversation of someone else is not found.
    pub async fn check_writable(
        mut self,
        subject: Subject,
        id: ConversationId,
    ) -> Result<Conversation, CoreError> {
        self.require(&subject, Action::ConversationWrite, Some(id))
            .await?;
        self.owned(&subject, id).await
    }

    pub async fn update(
        mut self,
        subject: Subject,
        id: ConversationId,
        command: UpdateConversation,
    ) -> Result<Conversation, CoreError> {
        self.require(&subject, Action::ConversationWrite, Some(id))
            .await?;
        let mut conversation = self.owned(&subject, id).await?;
        let now = Utc::now();
        if let Some(title) = command.title {
            conversation.title = valid_title(title)?;
        }
        match (command.archived, conversation.archived_at) {
            (Some(true), None) => conversation.archived_at = Some(now),
            (Some(false), Some(_)) => conversation.archived_at = None,
            _ => {}
        }
        conversation.updated_at = now;
        self.conversations.update(&conversation).await?;
        Ok(conversation)
    }

    /// Takes the turn slot of the conversation and stores the member's
    /// message, in one transaction: the message exists before the model is
    /// ever called. Without `message`, the last unanswered message is
    /// answered, which is how the first turn of a new conversation starts.
    pub async fn start_turn(
        mut self,
        subject: Subject,
        id: ConversationId,
        message: Option<String>,
    ) -> Result<StartedTurn, TurnError> {
        self.require(&subject, Action::ConversationWrite, Some(id))
            .await?;
        let conversation = self.owned(&subject, id).await?;
        if conversation.is_archived() {
            return Err(CoreError::Invalid("the conversation is archived".into()).into());
        }
        if conversation.is_full() {
            return Err(TurnError::Full);
        }
        let message = message.map(valid_message).transpose()?;
        if !self
            .conversations
            .begin_turn(id, Utc::now(), TURN_LEASE_SECS)
            .await?
        {
            return Err(TurnError::InProgress);
        }
        if let Some(text) = message {
            self.messages
                .append(
                    id,
                    MessageRole::User,
                    &MessageContent {
                        text,
                        ..MessageContent::default()
                    },
                    MessageStatus::Complete,
                )
                .await?;
        }
        let history = history_of(self.messages.list(id).await?);
        if history.last().map(|last| last.role) != Some(ChatRole::User) {
            return Err(CoreError::Invalid("there is no message to answer".into()).into());
        }
        Ok(StartedTurn {
            conversation,
            history,
        })
    }

    pub async fn finish_turn(
        mut self,
        subject: Subject,
        id: ConversationId,
        outcome: TurnOutcome,
    ) -> Result<(), CoreError> {
        let mut conversation = self.owned(&subject, id).await?;
        let kept = !outcome.content.text.is_empty()
            || !outcome.content.tool_calls.is_empty()
            || !outcome.content.cards.is_empty();
        if kept || outcome.status != MessageStatus::Complete {
            self.messages
                .append(id, MessageRole::Assistant, &outcome.content, outcome.status)
                .await?;
        }
        conversation.model = Some(outcome.model);
        conversation.input_tokens = conversation
            .input_tokens
            .saturating_add(outcome.usage.input_tokens);
        conversation.output_tokens = conversation
            .output_tokens
            .saturating_add(outcome.usage.output_tokens);
        conversation.cost_micros = conversation.cost_micros.saturating_add(outcome.cost_micros);
        conversation.updated_at = Utc::now();
        self.conversations.update(&conversation).await?;
        self.conversations.end_turn(id).await
    }

    /// Frees the turn slot when the turn could not even start its model call.
    pub async fn abandon_turn(
        mut self,
        subject: Subject,
        id: ConversationId,
    ) -> Result<(), CoreError> {
        self.owned(&subject, id).await?;
        self.conversations.end_turn(id).await
    }

    async fn owned(
        &mut self,
        subject: &Subject,
        id: ConversationId,
    ) -> Result<Conversation, CoreError> {
        self.conversations
            .find(subject.organization_id, &subject.id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    async fn require(
        &self,
        subject: &Subject,
        action: Action,
        id: Option<ConversationId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(
                subject.clone(),
                action,
                ResourceKind::Conversation,
                id.map(|id| id.0),
            ),
        )
        .await
    }
}

fn history_of(messages: Vec<Message>) -> Vec<ChatMessage> {
    messages
        .into_iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::User => ChatRole::User,
                MessageRole::Assistant => ChatRole::Assistant,
                MessageRole::Tool => return None,
            };
            let mut text = message.content.text;
            if text.is_empty() && role == ChatRole::Assistant {
                return None;
            }
            if message.status == MessageStatus::Interrupted && role == ChatRole::Assistant {
                text.push_str("\n[réponse interrompue]");
            }
            Some(ChatMessage { role, text })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        conversation::ports::{MockConversationRepository, MockMessageRepository},
        provider::ProviderKind,
    };

    fn subject(owner: &str) -> Subject {
        Subject {
            id: owner.into(),
            organization_id: OrganizationId::new(),
            actor: None,
            is_admin: false,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    fn stored(subject: &Subject, tokens: u64) -> Conversation {
        let now = Utc::now();
        Conversation {
            id: ConversationId::new(),
            organization_id: subject.organization_id,
            owner_subject: subject.id.clone(),
            title: "t".into(),
            model: None,
            input_tokens: tokens,
            output_tokens: 0,
            cost_micros: 0,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }

    fn message(conversation: &Conversation, seq: i32, role: MessageRole, text: &str) -> Message {
        Message {
            id: capitainerie_common::ConversationMessageId::new(),
            conversation_id: conversation.id,
            seq,
            role,
            content: MessageContent {
                text: text.into(),
                ..MessageContent::default()
            },
            status: MessageStatus::Complete,
            created_at: Utc::now(),
        }
    }

    fn finding(conversation: Conversation, begin: bool) -> MockConversationRepository {
        let mut conversations = MockConversationRepository::new();
        conversations
            .expect_find()
            .returning(move |org, owner, id| {
                let found = (conversation.organization_id == org
                    && conversation.owner_subject == owner
                    && conversation.id == id)
                    .then(|| conversation.clone());
                Box::pin(async move { Ok(found) })
            });
        conversations
            .expect_begin_turn()
            .returning(move |_, _, _| Box::pin(async move { Ok(begin) }));
        conversations
    }

    #[tokio::test]
    async fn creating_stores_the_conversation_and_its_first_message() {
        let owner = subject("user-1");
        let inserted = Arc::new(Mutex::new(Vec::new()));
        let mut conversations = MockConversationRepository::new();
        let sink = Arc::clone(&inserted);
        conversations
            .expect_insert()
            .returning(move |conversation| {
                sink.lock().unwrap().push(conversation.title.clone());
                Box::pin(async { Ok(()) })
            });
        let appended = Arc::new(Mutex::new(Vec::new()));
        let mut messages = MockMessageRepository::new();
        let sink = Arc::clone(&appended);
        messages
            .expect_append()
            .returning(move |id, role, content, status| {
                sink.lock()
                    .unwrap()
                    .push((role, content.text.clone(), status));
                let message = Message {
                    id: capitainerie_common::ConversationMessageId::new(),
                    conversation_id: id,
                    seq: 1,
                    role,
                    content: content.clone(),
                    status,
                    created_at: Utc::now(),
                };
                Box::pin(async move { Ok(message) })
            });
        let authorizer = allow();

        let conversation = ConversationService::new(conversations, messages, &authorizer)
            .create(owner.clone(), "  Ajoute un README\nau dépôt test ".into())
            .await
            .unwrap();

        assert_eq!(conversation.title, "Ajoute un README");
        assert_eq!(conversation.owner_subject, "user-1");
        assert_eq!(*inserted.lock().unwrap(), ["Ajoute un README"]);
        assert_eq!(
            *appended.lock().unwrap(),
            [(
                MessageRole::User,
                "Ajoute un README\nau dépôt test".to_owned(),
                MessageStatus::Complete
            )]
        );
    }

    #[tokio::test]
    async fn another_owner_finds_nothing() {
        let owner = subject("user-1");
        let conversation = stored(&owner, 0);
        let mut intruder = subject("user-2");
        intruder.organization_id = owner.organization_id;
        let authorizer = allow();

        let read = ConversationService::new(
            finding(conversation.clone(), true),
            MockMessageRepository::new(),
            &authorizer,
        )
        .get(intruder.clone(), conversation.id)
        .await;
        let turn = ConversationService::new(
            finding(conversation.clone(), true),
            MockMessageRepository::new(),
            &authorizer,
        )
        .start_turn(intruder, conversation.id, Some("salut".into()))
        .await;

        assert!(matches!(read, Err(CoreError::NotFound)));
        assert!(matches!(turn, Err(TurnError::Core(CoreError::NotFound))));
    }

    #[tokio::test]
    async fn a_second_turn_is_refused_while_the_first_holds_the_slot() {
        let owner = subject("user-1");
        let conversation = stored(&owner, 0);
        let authorizer = allow();

        let refused = ConversationService::new(
            finding(conversation.clone(), false),
            MockMessageRepository::new(),
            &authorizer,
        )
        .start_turn(owner, conversation.id, Some("salut".into()))
        .await;

        assert!(matches!(refused, Err(TurnError::InProgress)));
    }

    #[tokio::test]
    async fn a_full_conversation_asks_for_a_new_one() {
        let owner = subject("user-1");
        let conversation = stored(&owner, 400_000);
        let authorizer = allow();

        let refused = ConversationService::new(
            finding(conversation.clone(), true),
            MockMessageRepository::new(),
            &authorizer,
        )
        .start_turn(owner, conversation.id, Some("salut".into()))
        .await;

        assert!(matches!(refused, Err(TurnError::Full)));
    }

    #[tokio::test]
    async fn the_member_message_is_stored_and_the_history_ends_with_it() {
        let owner = subject("user-1");
        let conversation = stored(&owner, 10);
        let mut messages = MockMessageRepository::new();
        let first = message(&conversation, 1, MessageRole::User, "Bonjour");
        let answer = message(&conversation, 2, MessageRole::Assistant, "Salut");
        let second = message(&conversation, 3, MessageRole::User, "Un README");
        messages.expect_append().times(1).returning({
            let second = second.clone();
            move |_, _, _, _| {
                let second = second.clone();
                Box::pin(async move { Ok(second) })
            }
        });
        messages.expect_list().returning(move |_| {
            let all = vec![first.clone(), answer.clone(), second.clone()];
            Box::pin(async move { Ok(all) })
        });
        let authorizer = allow();

        let started =
            ConversationService::new(finding(conversation.clone(), true), messages, &authorizer)
                .start_turn(owner, conversation.id, Some("Un README".into()))
                .await
                .unwrap();

        let roles: Vec<_> = started.history.iter().map(|m| m.role).collect();
        assert_eq!(roles, [ChatRole::User, ChatRole::Assistant, ChatRole::User]);
        assert_eq!(started.history[2].text, "Un README");
    }

    #[tokio::test]
    async fn without_a_message_the_last_one_must_be_the_members() {
        let owner = subject("user-1");
        let conversation = stored(&owner, 0);
        let mut messages = MockMessageRepository::new();
        let answered = vec![
            message(&conversation, 1, MessageRole::User, "Bonjour"),
            message(&conversation, 2, MessageRole::Assistant, "Salut"),
        ];
        messages.expect_list().returning(move |_| {
            let all = answered.clone();
            Box::pin(async move { Ok(all) })
        });
        let authorizer = allow();

        let refused =
            ConversationService::new(finding(conversation.clone(), true), messages, &authorizer)
                .start_turn(owner, conversation.id, None)
                .await;

        assert!(matches!(
            refused,
            Err(TurnError::Core(CoreError::Invalid(_)))
        ));
    }

    #[tokio::test]
    async fn finishing_a_turn_keeps_a_partial_answer_and_adds_the_usage() {
        let owner = subject("user-1");
        let conversation = stored(&owner, 100);
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let mut messages = MockMessageRepository::new();
        let sink = Arc::clone(&recorded);
        messages
            .expect_append()
            .returning(move |id, role, content, status| {
                sink.lock()
                    .unwrap()
                    .push((role, status, content.text.clone()));
                let message = Message {
                    id: capitainerie_common::ConversationMessageId::new(),
                    conversation_id: id,
                    seq: 2,
                    role,
                    content: content.clone(),
                    status,
                    created_at: Utc::now(),
                };
                Box::pin(async move { Ok(message) })
            });
        let updated = Arc::new(Mutex::new(None));
        let mut conversations = finding(conversation.clone(), true);
        let sink = Arc::clone(&updated);
        conversations
            .expect_update()
            .returning(move |conversation| {
                *sink.lock().unwrap() = Some(conversation.clone());
                Box::pin(async { Ok(()) })
            });
        conversations
            .expect_end_turn()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = allow();

        ConversationService::new(conversations, messages, &authorizer)
            .finish_turn(
                owner,
                conversation.id,
                TurnOutcome {
                    content: MessageContent {
                        text: "Je cher".into(),
                        ..MessageContent::default()
                    },
                    status: MessageStatus::Interrupted,
                    usage: Usage {
                        model: "m".into(),
                        input_tokens: 40,
                        output_tokens: 7,
                    },
                    cost_micros: 12,
                    model: ModelRef {
                        provider: ProviderKind::Anthropic,
                        model: "m".into(),
                    },
                },
            )
            .await
            .unwrap();

        assert_eq!(
            *recorded.lock().unwrap(),
            [(
                MessageRole::Assistant,
                MessageStatus::Interrupted,
                "Je cher".to_owned()
            )]
        );
        let saved = updated.lock().unwrap().clone().unwrap();
        assert_eq!((saved.input_tokens, saved.output_tokens), (140, 7));
        assert_eq!(saved.cost_micros, 12);
        assert_eq!(saved.model.map(|m| m.model), Some("m".to_owned()));
    }

    #[test]
    fn an_interrupted_answer_is_marked_in_the_history_the_model_reads() {
        let owner = subject("user-1");
        let conversation = stored(&owner, 0);
        let mut cut = message(&conversation, 2, MessageRole::Assistant, "Je cher");
        cut.status = MessageStatus::Interrupted;
        let history = history_of(vec![
            message(&conversation, 1, MessageRole::User, "Bonjour"),
            cut,
            message(&conversation, 3, MessageRole::Assistant, ""),
        ]);
        assert_eq!(history.len(), 2);
        assert!(history[1].text.ends_with("[réponse interrompue]"));
    }
}
