use capitainerie_common::{
    CardId, ClientId, CoreError, IntegrationInstanceId, OrganizationId, RunId,
};

use crate::{
    domain::integration::{
        action::Access, intervention::Intervention, journal::Grant, ports::InterventionRepository,
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

pub struct PgInterventionRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgInterventionRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

fn write_error(error: sqlx::Error) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => CoreError::Invalid(
            "an instance of an intervention must belong to the client of its card".into(),
        ),
        other => map_sqlx_error(other),
    }
}

impl InterventionRepository for PgInterventionRepository<'_> {
    async fn insert(&mut self, intervention: &Intervention) -> Result<(), CoreError> {
        let instance_ids: Vec<_> = intervention
            .grants
            .iter()
            .map(|grant| grant.instance_id.0)
            .collect();
        let accesses: Vec<String> = intervention
            .grants
            .iter()
            .map(|grant| grant.access.as_str().to_owned())
            .collect();
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO interventions (
                run_id, organization_id, card_id, client_id, instructions, created_by, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            intervention.run_id.0,
            intervention.organization_id.0,
            intervention.card_id.0,
            intervention.client_id.0,
            intervention.instructions,
            intervention.created_by,
            intervention.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(write_error)?;
        sqlx::query!(
            r#"
            INSERT INTO intervention_grants (run_id, client_id, instance_id, access)
            SELECT $1, $2, granted.instance_id, granted.access
            FROM UNNEST($3::uuid[], $4::text[]) AS granted (instance_id, access)
            "#,
            intervention.run_id.0,
            intervention.client_id.0,
            &instance_ids,
            &accesses,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(write_error)
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Option<Intervention>, CoreError> {
        let mut tx = self.tx.lock().await;
        let Some(row) = sqlx::query!(
            r#"
            SELECT card_id, client_id, instructions, created_by, created_at
            FROM interventions
            WHERE organization_id = $1 AND run_id = $2
            "#,
            organization_id.0,
            run_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        else {
            return Ok(None);
        };
        let grants = sqlx::query!(
            r#"
            SELECT instance_id, access
            FROM intervention_grants
            WHERE run_id = $1
            ORDER BY instance_id
            "#,
            run_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(|grant| {
            Ok(Grant {
                instance_id: IntegrationInstanceId(grant.instance_id),
                access: Access::try_from(grant.access.as_str()).map_err(|error| {
                    CoreError::Internal(format!("stored intervention {run_id}: {error}"))
                })?,
            })
        })
        .collect::<Result<_, CoreError>>()?;
        Ok(Some(Intervention {
            run_id,
            organization_id,
            card_id: CardId(row.card_id),
            client_id: ClientId(row.client_id),
            instructions: row.instructions,
            grants,
            created_by: row.created_by,
            created_at: row.created_at,
        }))
    }
}
