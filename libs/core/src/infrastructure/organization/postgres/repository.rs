use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_macros::repository;

use super::model::OrganizationRow;
use crate::{
    Organization,
    domain::organization::ports::OrganizationRepository,
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = Organization, backend = Postgres)]
pub struct PgOrganizationRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgOrganizationRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl OrganizationRepository for PgOrganizationRepository<'_> {
    async fn find_by_issuer(
        &mut self,
        issuer_url: &str,
    ) -> Result<Option<Organization>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            OrganizationRow,
            r#"
            SELECT id, slug, name, issuer_url, created_at
            FROM organizations
            WHERE issuer_url = $1
            "#,
            issuer_url,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Organization::try_from)
        .transpose()
    }

    async fn find(&mut self, id: OrganizationId) -> Result<Option<Organization>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            OrganizationRow,
            "SELECT id, slug, name, issuer_url, created_at FROM organizations WHERE id = $1",
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Organization::try_from)
        .transpose()
    }
}
