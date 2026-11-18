use capitainerie_common::{AgentId, CoreError, OrganizationId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::budget::{Budget, BudgetSettings, Limit, Spend, ports::BudgetRepository},
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Budget, backend = Postgres)]
pub struct PgBudgetRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgBudgetRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl BudgetRepository for PgBudgetRepository<'_> {
    async fn list(&mut self, organization_id: OrganizationId) -> Result<Vec<Budget>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT b.id, b.organization_id, b.agent_id, a.name AS "agent_name?",
                   b.monthly_limit_cents, b.alert_threshold_percent, b.updated_by, b.updated_at
            FROM budgets b
            LEFT JOIN agents a ON a.id = b.agent_id
            WHERE b.organization_id = $1
            "#,
            organization_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| Budget {
                id: row.id,
                organization_id: OrganizationId(row.organization_id),
                agent_id: row.agent_id.map(AgentId),
                agent_name: row.agent_name,
                limit: Limit {
                    monthly_limit_cents: row.monthly_limit_cents,
                    alert_threshold_percent: row.alert_threshold_percent,
                },
                updated_by: row.updated_by,
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn replace(
        &mut self,
        organization_id: OrganizationId,
        settings: &BudgetSettings,
        by: &str,
        now: DateTime<Utc>,
    ) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "DELETE FROM budgets WHERE organization_id = $1",
            organization_id.0
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        let scopes = settings
            .organization
            .map(|limit| (None, limit))
            .into_iter()
            .chain(
                settings
                    .agents
                    .iter()
                    .map(|(agent, limit)| (Some(*agent), *limit)),
            );
        for (agent, limit) in scopes {
            sqlx::query!(
                r#"
                INSERT INTO budgets (id, organization_id, agent_id, monthly_limit_cents,
                                     alert_threshold_percent, updated_by, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                Uuid::now_v7(),
                organization_id.0,
                agent.map(|id: AgentId| id.0),
                limit.monthly_limit_cents,
                limit.alert_threshold_percent,
                by,
                now,
            )
            .execute(&mut ***tx)
            .await
            .map_err(map_sqlx_error)?;
        }
        Ok(())
    }

    async fn count_agents(
        &mut self,
        organization_id: OrganizationId,
        agents: &[AgentId],
    ) -> Result<i64, CoreError> {
        let ids: Vec<Uuid> = agents.iter().map(|id| id.0).collect();
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM agents WHERE organization_id = $1 AND id = ANY($2)"#,
            organization_id.0,
            &ids,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn spend(
        &mut self,
        organization_id: OrganizationId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Spend, CoreError> {
        let mut tx = self.tx.lock().await;
        let total_micros = sqlx::query_scalar!(
            r#"
            SELECT COALESCE(sum(cost_micros), 0)::bigint AS "total!"
            FROM run_steps
            WHERE organization_id = $1 AND started_at >= $2 AND started_at < $3
            "#,
            organization_id.0,
            from,
            to,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        let by_agent = sqlx::query!(
            r#"
            SELECT t.agent_id, COALESCE(sum(s.cost_micros), 0)::bigint AS "spent!"
            FROM run_steps s
            JOIN agent_tasks t ON t.run_id = s.run_id
            WHERE s.organization_id = $1 AND s.started_at >= $2 AND s.started_at < $3
            GROUP BY t.agent_id
            "#,
            organization_id.0,
            from,
            to,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(|row| (AgentId(row.agent_id), row.spent))
        .collect();
        Ok(Spend {
            total_micros,
            by_agent,
        })
    }
}
