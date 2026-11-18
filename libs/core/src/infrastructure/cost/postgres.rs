use capitainerie_common::{AgentId, CardId, CoreError, OrganizationId, ProjectId};
use capitainerie_macros::repository;

use crate::{
    domain::cost::{
        AgentRef, AgentRow, CostFilter, DaySpend, Period, ProjectAgentRow, Spent, TaskAgentRow,
        UserModelRow, ports::CostRepository,
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Cost, backend = Postgres)]
pub struct PgCostRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgCostRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

fn agent_ref(id: Option<uuid::Uuid>, name: String) -> AgentRef {
    AgentRef {
        id: id.map(AgentId),
        name,
    }
}

fn spent(input_tokens: i64, output_tokens: i64, cost_micros: i64) -> Spent {
    Spent {
        input_tokens,
        output_tokens,
        cost_micros,
    }
}

impl CostRepository for PgCostRepository<'_> {
    async fn project_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> Result<Vec<ProjectAgentRow>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT p.id AS "project_id?", p.name AS "project_name?",
                   a.id AS "agent_id?", COALESCE(a.name, s.agent, 'unknown') AS "agent_name!",
                   count(DISTINCT s.run_id) AS "runs!",
                   COALESCE(sum(s.input_tokens), 0)::bigint AS "input_tokens!",
                   COALESCE(sum(s.output_tokens), 0)::bigint AS "output_tokens!",
                   COALESCE(sum(s.cost_micros), 0)::bigint AS "cost_micros!"
            FROM run_steps s
            JOIN runs r ON r.id = s.run_id AND r.organization_id = s.organization_id
            LEFT JOIN agent_tasks t ON t.run_id = s.run_id AND t.organization_id = s.organization_id
            LEFT JOIN agents a ON a.id = t.agent_id AND a.organization_id = s.organization_id
            LEFT JOIN cards c ON c.id = r.card_id AND c.organization_id = s.organization_id
            LEFT JOIN transcripts tr ON tr.id = r.transcript_id AND tr.organization_id = s.organization_id
            LEFT JOIN projects p ON p.id = COALESCE(c.project_id, tr.project_id)
                 AND p.organization_id = s.organization_id
            WHERE s.organization_id = $1 AND s.started_at >= $2 AND s.started_at < $3
              AND ($4::uuid IS NULL OR a.id = $4) AND ($5::uuid IS NULL OR r.card_id = $5)
            GROUP BY p.id, p.name, a.id, COALESCE(a.name, s.agent, 'unknown')
            ORDER BY 8 DESC, p.name, 4, a.id
            "#,
            organization_id.0,
            period.from(),
            period.to(),
            filter.agent.map(|id| id.0),
            filter.card.map(|id| id.0),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| ProjectAgentRow {
                project_id: row.project_id.map(ProjectId),
                project_name: row.project_name,
                agent: agent_ref(row.agent_id, row.agent_name),
                runs: row.runs,
                spent: spent(row.input_tokens, row.output_tokens, row.cost_micros),
            })
            .collect())
    }

    async fn agent_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> Result<Vec<AgentRow>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT a.id AS "agent_id?", COALESCE(a.name, s.agent, 'unknown') AS "agent_name!",
                   count(DISTINCT s.run_id) AS "runs!",
                   COALESCE(sum(s.input_tokens), 0)::bigint AS "input_tokens!",
                   COALESCE(sum(s.output_tokens), 0)::bigint AS "output_tokens!",
                   COALESCE(sum(s.cost_micros), 0)::bigint AS "cost_micros!"
            FROM run_steps s
            JOIN runs r ON r.id = s.run_id AND r.organization_id = s.organization_id
            LEFT JOIN agent_tasks t ON t.run_id = s.run_id AND t.organization_id = s.organization_id
            LEFT JOIN agents a ON a.id = t.agent_id AND a.organization_id = s.organization_id
            WHERE s.organization_id = $1 AND s.started_at >= $2 AND s.started_at < $3
              AND ($4::uuid IS NULL OR a.id = $4) AND ($5::uuid IS NULL OR r.card_id = $5)
            GROUP BY a.id, COALESCE(a.name, s.agent, 'unknown')
            ORDER BY 6 DESC, 2, a.id
            "#,
            organization_id.0,
            period.from(),
            period.to(),
            filter.agent.map(|id| id.0),
            filter.card.map(|id| id.0),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| AgentRow {
                agent: agent_ref(row.agent_id, row.agent_name),
                runs: row.runs,
                spent: spent(row.input_tokens, row.output_tokens, row.cost_micros),
            })
            .collect())
    }

    async fn task_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        project: Option<ProjectId>,
        filter: &CostFilter,
    ) -> Result<Vec<TaskAgentRow>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT c.id AS "card_id!", c.title AS "title!", c.project_id AS "project_id?",
                   a.id AS "agent_id?", COALESCE(a.name, s.agent, 'unknown') AS "agent_name!",
                   count(DISTINCT s.run_id) AS "runs!",
                   COALESCE(sum(s.input_tokens), 0)::bigint AS "input_tokens!",
                   COALESCE(sum(s.output_tokens), 0)::bigint AS "output_tokens!",
                   COALESCE(sum(s.cost_micros), 0)::bigint AS "cost_micros!"
            FROM run_steps s
            JOIN runs r ON r.id = s.run_id AND r.organization_id = s.organization_id
            LEFT JOIN agent_tasks t ON t.run_id = s.run_id AND t.organization_id = s.organization_id
            LEFT JOIN agents a ON a.id = t.agent_id AND a.organization_id = s.organization_id
            LEFT JOIN cards c ON c.id = r.card_id AND c.organization_id = s.organization_id
            WHERE s.organization_id = $1 AND s.started_at >= $2 AND s.started_at < $3
              AND r.card_id IS NOT NULL
              AND ($4::uuid IS NULL OR c.project_id = $4)
              AND ($5::uuid IS NULL OR a.id = $5) AND ($6::uuid IS NULL OR r.card_id = $6)
            GROUP BY c.id, c.title, c.project_id, a.id, COALESCE(a.name, s.agent, 'unknown')
            ORDER BY 9 DESC, c.title, c.id, 5, a.id
            "#,
            organization_id.0,
            period.from(),
            period.to(),
            project.map(|id| id.0),
            filter.agent.map(|id| id.0),
            filter.card.map(|id| id.0),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| TaskAgentRow {
                card_id: CardId(row.card_id),
                title: row.title,
                project_id: row.project_id.map(ProjectId),
                agent: agent_ref(row.agent_id, row.agent_name),
                runs: row.runs,
                spent: spent(row.input_tokens, row.output_tokens, row.cost_micros),
            })
            .collect())
    }

    async fn user_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> Result<Vec<UserModelRow>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT r.initiated_by AS "user!", COALESCE(s.model, 'unknown') AS "model!",
                   COALESCE(sum(s.input_tokens), 0)::bigint AS "input_tokens!",
                   COALESCE(sum(s.output_tokens), 0)::bigint AS "output_tokens!",
                   COALESCE(sum(s.cost_micros), 0)::bigint AS "cost_micros!"
            FROM run_steps s
            JOIN runs r ON r.id = s.run_id AND r.organization_id = s.organization_id
            LEFT JOIN agent_tasks t ON t.run_id = s.run_id AND t.organization_id = s.organization_id
            LEFT JOIN agents a ON a.id = t.agent_id AND a.organization_id = s.organization_id
            WHERE s.organization_id = $1 AND s.started_at >= $2 AND s.started_at < $3
              AND ($4::uuid IS NULL OR a.id = $4) AND ($5::uuid IS NULL OR r.card_id = $5)
            GROUP BY r.initiated_by, COALESCE(s.model, 'unknown')
            ORDER BY 5 DESC, 1, 2
            "#,
            organization_id.0,
            period.from(),
            period.to(),
            filter.agent.map(|id| id.0),
            filter.card.map(|id| id.0),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| UserModelRow {
                user: row.user,
                model: row.model,
                spent: spent(row.input_tokens, row.output_tokens, row.cost_micros),
            })
            .collect())
    }

    async fn day_rows(
        &mut self,
        organization_id: OrganizationId,
        period: &Period,
        filter: &CostFilter,
    ) -> Result<Vec<DaySpend>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT (s.started_at AT TIME ZONE 'UTC')::date AS "day!",
                   COALESCE(sum(s.input_tokens), 0)::bigint AS "input_tokens!",
                   COALESCE(sum(s.output_tokens), 0)::bigint AS "output_tokens!",
                   COALESCE(sum(s.cost_micros), 0)::bigint AS "cost_micros!"
            FROM run_steps s
            JOIN runs r ON r.id = s.run_id AND r.organization_id = s.organization_id
            LEFT JOIN agent_tasks t ON t.run_id = s.run_id AND t.organization_id = s.organization_id
            LEFT JOIN agents a ON a.id = t.agent_id AND a.organization_id = s.organization_id
            WHERE s.organization_id = $1 AND s.started_at >= $2 AND s.started_at < $3
              AND ($4::uuid IS NULL OR a.id = $4) AND ($5::uuid IS NULL OR r.card_id = $5)
            GROUP BY 1
            ORDER BY 1
            "#,
            organization_id.0,
            period.from(),
            period.to(),
            filter.agent.map(|id| id.0),
            filter.card.map(|id| id.0),
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| DaySpend {
                day: row.day,
                spent: spent(row.input_tokens, row.output_tokens, row.cost_micros),
            })
            .collect())
    }

    async fn project_exists(
        &mut self,
        organization_id: OrganizationId,
        project_id: ProjectId,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM projects WHERE organization_id = $1 AND id = $2) AS "exists!""#,
            organization_id.0,
            project_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn agent_exists(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM agents WHERE organization_id = $1 AND id = $2) AS "exists!""#,
            organization_id.0,
            agent_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }

    async fn card_exists(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM cards WHERE organization_id = $1 AND id = $2) AS "exists!""#,
            organization_id.0,
            card_id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }
}
