use capitainerie_common::{AgentId, ApprovalId, CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    domain::approval::{Approval, ApprovalKind, ApprovalStatus, ports::ApprovalRepository},
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const LIST_LIMIT: i64 = 200;

#[repository(domain = Approval, backend = Postgres)]
pub struct PgApprovalRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgApprovalRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl ApprovalRepository for PgApprovalRepository<'_> {
    async fn insert(&mut self, approval: &Approval) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO approvals (
                id, organization_id, kind, title, detail, requested_by_agent_id, card_id, run_id,
                status, decided_by, decided_at, decision_note, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            approval.id.0,
            approval.organization_id.0,
            approval.kind.as_str(),
            approval.title,
            approval.detail,
            approval.requested_by_agent_id.map(|id| id.0),
            approval.card_id.map(|id| id.0),
            approval.run_id.map(|id| id.0),
            approval.status.as_str(),
            approval.decided_by,
            approval.decided_at,
            approval.decision_note,
            approval.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ApprovalId,
    ) -> Result<Option<Approval>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ApprovalRow,
            r#"
            SELECT id, organization_id, kind, title, detail, requested_by_agent_id, card_id,
                   run_id, status, decided_by, decided_at, decision_note, created_at
            FROM approvals
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Approval::try_from)
        .transpose()
    }

    async fn save_decision(&mut self, approval: &Approval) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE approvals
            SET status = $3, decided_by = $4, decided_at = $5, decision_note = $6
            WHERE organization_id = $1 AND id = $2 AND status = 'pending'
            "#,
            approval.organization_id.0,
            approval.id.0,
            approval.status.as_str(),
            approval.decided_by,
            approval.decided_at,
            approval.decision_note,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(updated.rows_affected() == 1)
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        status: Option<ApprovalStatus>,
    ) -> Result<Vec<Approval>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ApprovalRow,
            r#"
            SELECT id, organization_id, kind, title, detail, requested_by_agent_id, card_id,
                   run_id, status, decided_by, decided_at, decision_note, created_at
            FROM approvals
            WHERE organization_id = $1 AND ($2::text IS NULL OR status = $2)
            ORDER BY created_at DESC, id DESC
            LIMIT $3
            "#,
            organization_id.0,
            status.map(ApprovalStatus::as_str),
            LIST_LIMIT,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Approval::try_from)
        .collect()
    }

    async fn count_pending(&mut self, organization_id: OrganizationId) -> Result<i64, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"
            SELECT count(*) AS "count!"
            FROM approvals
            WHERE organization_id = $1 AND status = 'pending'
            "#,
            organization_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }
}

struct ApprovalRow {
    id: Uuid,
    organization_id: Uuid,
    kind: String,
    title: String,
    detail: Value,
    requested_by_agent_id: Option<Uuid>,
    card_id: Option<Uuid>,
    run_id: Option<Uuid>,
    status: String,
    decided_by: Option<String>,
    decided_at: Option<DateTime<Utc>>,
    decision_note: Option<String>,
    created_at: DateTime<Utc>,
}

impl TryFrom<ApprovalRow> for Approval {
    type Error = CoreError;

    fn try_from(row: ApprovalRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored approval {}: {error}", row.id));
        Ok(Self {
            id: ApprovalId(row.id),
            organization_id: OrganizationId(row.organization_id),
            kind: ApprovalKind::try_from(row.kind.as_str()).map_err(stored)?,
            title: row.title,
            detail: row.detail,
            requested_by_agent_id: row.requested_by_agent_id.map(AgentId),
            card_id: row.card_id.map(CardId),
            run_id: row.run_id.map(RunId),
            status: ApprovalStatus::try_from(row.status.as_str()).map_err(stored)?,
            decided_by: row.decided_by,
            decided_at: row.decided_at,
            decision_note: row.decision_note,
            created_at: row.created_at,
        })
    }
}
