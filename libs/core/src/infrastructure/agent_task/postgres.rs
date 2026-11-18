use capitainerie_common::{AgentId, CardId, CoreError, OrganizationId, RunId};

use crate::{
    domain::{
        agent_task::{AgentTask, ports::AgentTaskRepository},
        provider::{ModelRef, ProviderKind},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

pub struct PgAgentTaskRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgAgentTaskRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl AgentTaskRepository for PgAgentTaskRepository<'_> {
    async fn insert(&mut self, task: &AgentTask) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO agent_tasks (run_id, organization_id, card_id, agent_id, allow_writes, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            task.run_id.0,
            task.organization_id.0,
            task.card_id.0,
            task.agent_id.0,
            task.allow_writes,
            task.created_by,
            task.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Option<AgentTask>, CoreError> {
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT card_id, agent_id, allow_writes, provider, model, created_by, created_at
            FROM agent_tasks
            WHERE organization_id = $1 AND run_id = $2
            "#,
            organization_id.0,
            run_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            let model = match (row.provider, row.model) {
                (Some(provider), Some(model)) => Some(ModelRef {
                    provider: ProviderKind::try_from(provider.as_str())?,
                    model,
                }),
                _ => None,
            };
            Ok(AgentTask {
                run_id,
                organization_id,
                card_id: CardId(row.card_id),
                agent_id: AgentId(row.agent_id),
                allow_writes: row.allow_writes,
                model,
                created_by: row.created_by,
                created_at: row.created_at,
            })
        })
        .transpose()
    }

    async fn freeze_model(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
        model: &ModelRef,
    ) -> Result<ModelRef, CoreError> {
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            UPDATE agent_tasks
            SET provider = COALESCE(provider, $3), model = COALESCE(model, $4)
            WHERE organization_id = $1 AND run_id = $2
            RETURNING provider AS "provider!", model AS "model!"
            "#,
            organization_id.0,
            run_id.0,
            model.provider.as_str(),
            model.model,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(CoreError::NotFound)?;
        Ok(ModelRef {
            provider: ProviderKind::try_from(row.provider.as_str())?,
            model: row.model,
        })
    }
}
