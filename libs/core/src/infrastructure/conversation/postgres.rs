use capitainerie_common::{ConversationId, CoreError, OrganizationId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use sqlx::types::Json;

use super::model::{ConversationRow, MessageRow, stored_counter};
use crate::{
    domain::conversation::{
        Conversation, Message, MessageContent, MessageRole, MessageStatus,
        ports::{ConversationRepository, MessageRepository},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Conversation, backend = Postgres)]
pub struct PgConversationRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgConversationRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl ConversationRepository for PgConversationRepository<'_> {
    async fn insert(&mut self, conversation: &Conversation) -> Result<(), CoreError> {
        let input = stored_counter(conversation.input_tokens, "input tokens")?;
        let output = stored_counter(conversation.output_tokens, "output tokens")?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO conversations (
                id, organization_id, owner_subject, title, model,
                input_tokens, output_tokens, cost_micros, created_at, updated_at, archived_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            conversation.id.0,
            conversation.organization_id.0,
            conversation.owner_subject,
            conversation.title,
            conversation.model.as_ref().map(Json)
                as Option<Json<&crate::domain::provider::ModelRef>>,
            input,
            output,
            conversation.cost_micros,
            conversation.created_at,
            conversation.updated_at,
            conversation.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        owner_subject: &str,
        id: ConversationId,
    ) -> Result<Option<Conversation>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, organization_id, owner_subject, title,
                   model AS "model: Json<crate::domain::provider::ModelRef>",
                   input_tokens, output_tokens, cost_micros, created_at, updated_at, archived_at
            FROM conversations
            WHERE organization_id = $1 AND owner_subject = $2 AND id = $3
            "#,
            organization_id.0,
            owner_subject,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Conversation::try_from)
        .transpose()
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        owner_subject: &str,
    ) -> Result<Vec<Conversation>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, organization_id, owner_subject, title,
                   model AS "model: Json<crate::domain::provider::ModelRef>",
                   input_tokens, output_tokens, cost_micros, created_at, updated_at, archived_at
            FROM conversations
            WHERE organization_id = $1 AND owner_subject = $2 AND archived_at IS NULL
            ORDER BY updated_at DESC, id DESC
            LIMIT 200
            "#,
            organization_id.0,
            owner_subject,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Conversation::try_from)
        .collect()
    }

    async fn update(&mut self, conversation: &Conversation) -> Result<(), CoreError> {
        let input = stored_counter(conversation.input_tokens, "input tokens")?;
        let output = stored_counter(conversation.output_tokens, "output tokens")?;
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE conversations
            SET title = $3, model = $4, input_tokens = $5, output_tokens = $6,
                cost_micros = $7, updated_at = $8, archived_at = $9
            WHERE organization_id = $1 AND id = $2
            "#,
            conversation.organization_id.0,
            conversation.id.0,
            conversation.title,
            conversation.model.as_ref().map(Json)
                as Option<Json<&crate::domain::provider::ModelRef>>,
            input,
            output,
            conversation.cost_micros,
            conversation.updated_at,
            conversation.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn begin_turn(
        &mut self,
        id: ConversationId,
        now: DateTime<Utc>,
        lease_secs: i64,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        let taken = sqlx::query!(
            r#"
            UPDATE conversations
            SET turn_started_at = $2
            WHERE id = $1
              AND (turn_started_at IS NULL
                   OR turn_started_at < $2::timestamptz - make_interval(secs => $3::double precision))
            "#,
            id.0,
            now,
            lease_secs as f64,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(taken.rows_affected() == 1)
    }

    async fn end_turn(&mut self, id: ConversationId) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "UPDATE conversations SET turn_started_at = NULL WHERE id = $1",
            id.0
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}

#[repository(domain = ConversationMessage, backend = Postgres)]
pub struct PgMessageRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgMessageRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl MessageRepository for PgMessageRepository<'_> {
    async fn append(
        &mut self,
        conversation_id: ConversationId,
        role: MessageRole,
        content: &MessageContent,
        status: MessageStatus,
    ) -> Result<Message, CoreError> {
        let id = capitainerie_common::ConversationMessageId::new();
        let mut tx = self.tx.lock().await;
        let row = sqlx::query_as!(
            MessageRow,
            r#"
            INSERT INTO conversation_messages (id, conversation_id, seq, role, content, status, created_at)
            SELECT $1, $2, COALESCE(MAX(seq), 0) + 1, $3, $4, $5, now()
            FROM conversation_messages
            WHERE conversation_id = $2
            RETURNING id AS "id!", conversation_id AS "conversation_id!", seq AS "seq!", role AS "role!",
                      content AS "content!: Json<MessageContent>", status AS "status!",
                      created_at AS "created_at!"
            "#,
            id.0,
            conversation_id.0,
            role.as_str(),
            Json(content) as Json<&MessageContent>,
            status.as_str(),
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Message::try_from(row)
    }

    async fn list(&mut self, conversation_id: ConversationId) -> Result<Vec<Message>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            MessageRow,
            r#"
            SELECT id, conversation_id, seq, role, content AS "content: Json<MessageContent>",
                   status, created_at
            FROM conversation_messages
            WHERE conversation_id = $1
            ORDER BY seq
            "#,
            conversation_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Message::try_from)
        .collect()
    }
}
