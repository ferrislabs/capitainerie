use capitainerie_common::{
    CardId, CoreError, IntegrationActionId, IntegrationInstanceId, OrganizationId, RunId,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    domain::integration::{
        action::Access,
        journal::{IntegrationAction, Outcome},
        ports::IntegrationActionRepository,
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
};

const MISSING_KEY: &str = "SECRETS_KEY is not configured";

pub struct PgIntegrationActionRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgIntegrationActionRepository<'tx, 'c> {
    pub fn new(tx: &SharedTx<'tx>, cipher: Option<&'c SecretCipher>) -> Self {
        Self {
            tx: tx.clone(),
            cipher,
        }
    }

    fn cipher(&self) -> Result<&'c SecretCipher, CoreError> {
        self.cipher
            .ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))
    }
}

fn binding(organization_id: OrganizationId, id: IntegrationActionId) -> Vec<u8> {
    format!("{organization_id}/integration_action/{id}/snapshot").into_bytes()
}

impl IntegrationActionRepository for PgIntegrationActionRepository<'_, '_> {
    async fn insert(
        &mut self,
        action: &IntegrationAction,
        snapshot: Option<&Value>,
    ) -> Result<(), CoreError> {
        let sealed = snapshot
            .map(|snapshot| {
                let plaintext = serde_json::to_vec(snapshot).map_err(|error| {
                    CoreError::Internal(format!("a snapshot cannot be serialized: {error}"))
                })?;
                self.cipher()?
                    .seal(&plaintext, &binding(action.organization_id, action.id))
            })
            .transpose()?;
        let (nonce, ciphertext) = match sealed {
            Some(Sealed { nonce, ciphertext }) => (Some(nonce), Some(ciphertext)),
            None => (None, None),
        };
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO integration_actions (
                id, organization_id, run_id, card_id, instance_id, subject, actor, tool, access,
                target, arguments, outcome, reason_code, reason, state_before, state_after,
                snapshot_nonce, snapshot_ciphertext, revertible, reverts_action_id,
                reverted_by_action_id, latency_ms, created_at, finished_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
                $19, $20, $21, $22, $23, $24
            )
            "#,
            action.id.0,
            action.organization_id.0,
            action.run_id.map(|id| id.0),
            action.card_id.map(|id| id.0),
            action.instance_id.0,
            action.subject,
            action.actor,
            action.tool,
            action.access.as_str(),
            action.target,
            action.arguments,
            action.outcome.as_str(),
            action.reason_code,
            action.reason,
            action.before,
            action.after,
            nonce,
            ciphertext,
            action.revertible,
            action.reverts_action_id.map(|id| id.0),
            action.reverted_by_action_id.map(|id| id.0),
            action.latency_ms,
            action.created_at,
            action.finished_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn complete(&mut self, action: &IntegrationAction) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE integration_actions
            SET outcome = $3, reason_code = $4, reason = $5, state_after = $6, revertible = $7,
                latency_ms = $8, finished_at = $9
            WHERE organization_id = $1 AND id = $2 AND outcome = 'pending'
            "#,
            action.organization_id.0,
            action.id.0,
            action.outcome.as_str(),
            action.reason_code,
            action.reason,
            action.after,
            action.revertible,
            action.latency_ms,
            action.finished_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
    ) -> Result<Option<IntegrationAction>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ActionRow,
            r#"
            SELECT id, organization_id, run_id, card_id, instance_id, subject, actor, tool,
                   access, target, arguments, outcome, reason_code, reason, state_before,
                   state_after, revertible, reverts_action_id, reverted_by_action_id,
                   latency_ms, created_at, finished_at
            FROM integration_actions
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(IntegrationAction::try_from)
        .transpose()
    }

    async fn snapshot(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
    ) -> Result<Option<Value>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT snapshot_nonce, snapshot_ciphertext
            FROM integration_actions
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        let Some((nonce, ciphertext)) =
            row.and_then(|row| row.snapshot_nonce.zip(row.snapshot_ciphertext))
        else {
            return Ok(None);
        };
        let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
        let plaintext =
            cipher.open(&Sealed { nonce, ciphertext }, &binding(organization_id, id))?;
        serde_json::from_slice(&plaintext)
            .map(Some)
            .map_err(|error| {
                CoreError::Internal(format!(
                    "the stored snapshot of action {id} is not JSON: {error}"
                ))
            })
    }

    async fn list_for_run(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Vec<IntegrationAction>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ActionRow,
            r#"
            SELECT id, organization_id, run_id, card_id, instance_id, subject, actor, tool,
                   access, target, arguments, outcome, reason_code, reason, state_before,
                   state_after, revertible, reverts_action_id, reverted_by_action_id,
                   latency_ms, created_at, finished_at
            FROM integration_actions
            WHERE organization_id = $1 AND run_id = $2
            ORDER BY created_at, id
            "#,
            organization_id.0,
            run_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(IntegrationAction::try_from)
        .collect()
    }

    async fn list_for_card(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> Result<Vec<IntegrationAction>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ActionRow,
            r#"
            SELECT id, organization_id, run_id, card_id, instance_id, subject, actor, tool,
                   access, target, arguments, outcome, reason_code, reason, state_before,
                   state_after, revertible, reverts_action_id, reverted_by_action_id,
                   latency_ms, created_at, finished_at
            FROM integration_actions
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
        .map(IntegrationAction::try_from)
        .collect()
    }

    async fn mark_reverted(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
        by: IntegrationActionId,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE integration_actions
            SET reverted_by_action_id = $3
            WHERE organization_id = $1 AND id = $2 AND reverted_by_action_id IS NULL
            "#,
            organization_id.0,
            id.0,
            by.0,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(updated.rows_affected() > 0)
    }

    async fn count_writes_not_denied(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<i64, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"
            SELECT count(*) AS "count!"
            FROM integration_actions
            WHERE organization_id = $1 AND run_id = $2 AND access = 'write'
              AND outcome IN ('pending', 'succeeded', 'failed')
              AND reason_code IS DISTINCT FROM 'before_unreadable'
            "#,
            organization_id.0,
            run_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }
}

struct ActionRow {
    id: Uuid,
    organization_id: Uuid,
    run_id: Option<Uuid>,
    card_id: Option<Uuid>,
    instance_id: Uuid,
    subject: String,
    actor: String,
    tool: String,
    access: String,
    target: Value,
    arguments: Value,
    outcome: String,
    reason_code: Option<String>,
    reason: Option<String>,
    state_before: Option<Value>,
    state_after: Option<Value>,
    revertible: bool,
    reverts_action_id: Option<Uuid>,
    reverted_by_action_id: Option<Uuid>,
    latency_ms: Option<i64>,
    created_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}

impl TryFrom<ActionRow> for IntegrationAction {
    type Error = CoreError;

    fn try_from(row: ActionRow) -> Result<Self, Self::Error> {
        let stored = |error: CoreError| {
            CoreError::Internal(format!("stored integration action {}: {error}", row.id))
        };
        Ok(Self {
            id: IntegrationActionId(row.id),
            organization_id: OrganizationId(row.organization_id),
            run_id: row.run_id.map(RunId),
            card_id: row.card_id.map(CardId),
            instance_id: IntegrationInstanceId(row.instance_id),
            subject: row.subject,
            actor: row.actor,
            tool: row.tool,
            access: Access::try_from(row.access.as_str()).map_err(stored)?,
            target: row.target,
            arguments: row.arguments,
            outcome: Outcome::try_from(row.outcome.as_str()).map_err(stored)?,
            reason_code: row.reason_code,
            reason: row.reason,
            before: row.state_before,
            after: row.state_after,
            revertible: row.revertible,
            reverts_action_id: row.reverts_action_id.map(IntegrationActionId),
            reverted_by_action_id: row.reverted_by_action_id.map(IntegrationActionId),
            latency_ms: row.latency_ms,
            created_at: row.created_at,
            finished_at: row.finished_at,
        })
    }
}
