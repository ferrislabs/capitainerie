use capitainerie_common::{CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;

use crate::{
    domain::authorization::{DecisionRecord, ports::DecisionLogRepository},
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = DecisionLog, backend = Postgres)]
pub struct PgDecisionLogRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgDecisionLogRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl DecisionLogRepository for PgDecisionLogRepository<'_> {
    async fn list_for_run(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Vec<DecisionRecord>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            DecisionRecord,
            r#"
            SELECT subject, actor, action, resource_kind, resource_id, allowed, reason, decided_at
            FROM authorization_decisions
            WHERE organization_id = $1 AND run_id = $2
            ORDER BY decided_at, id
            "#,
            organization_id.0,
            run_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }
}
