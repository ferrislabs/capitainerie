use capitainerie_common::{CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};

use crate::{
    domain::execution::{
        GithubInstallation, TaskGrant,
        ports::{GithubInstallationRepository, TaskGrantRepository},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

#[repository(domain = TaskGrant, backend = Postgres)]
pub struct PgTaskGrantRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgTaskGrantRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl TaskGrantRepository for PgTaskGrantRepository<'_> {
    async fn insert(&mut self, token_sha256: [u8; 32], grant: &TaskGrant) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO task_grants (token_sha256, run_id, organization_id, repository, branch, expires_at, revoked_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            token_sha256.as_slice(),
            grant.run_id.0,
            grant.organization_id.0,
            grant.repository.to_string(),
            grant.branch,
            grant.expires_at,
            grant.revoked_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn find(&mut self, token_sha256: [u8; 32]) -> Result<Option<TaskGrant>, CoreError> {
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT run_id, organization_id, repository, branch, expires_at, revoked_at
            FROM task_grants
            WHERE token_sha256 = $1
            "#,
            token_sha256.as_slice(),
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            Ok(TaskGrant {
                run_id: RunId(row.run_id),
                organization_id: OrganizationId(row.organization_id),
                repository: row.repository.parse()?,
                branch: row.branch,
                expires_at: row.expires_at,
                revoked_at: row.revoked_at,
            })
        })
        .transpose()
    }

    async fn revoke(&mut self, run_id: RunId, now: DateTime<Utc>) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "UPDATE task_grants SET revoked_at = $2 WHERE run_id = $1 AND revoked_at IS NULL",
            run_id.0,
            now,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}

#[repository(domain = GithubInstallation, backend = Postgres)]
pub struct PgGithubInstallationRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgGithubInstallationRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl GithubInstallationRepository for PgGithubInstallationRepository<'_> {
    async fn get(
        &mut self,
        organization_id: OrganizationId,
    ) -> Result<Option<GithubInstallation>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            SELECT installation_id, account_login, created_by, created_at
            FROM github_installations
            WHERE organization_id = $1
            "#,
            organization_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map(|row| {
            row.map(|row| GithubInstallation {
                organization_id,
                installation_id: row.installation_id,
                account_login: row.account_login,
                created_by: row.created_by,
                created_at: row.created_at,
            })
        })
        .map_err(map_sqlx_error)
    }

    async fn save(&mut self, installation: &GithubInstallation) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO github_installations (organization_id, installation_id, account_login, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (organization_id) DO UPDATE
            SET installation_id = EXCLUDED.installation_id, account_login = EXCLUDED.account_login,
                created_by = EXCLUDED.created_by, created_at = EXCLUDED.created_at
            "#,
            installation.organization_id.0,
            installation.installation_id,
            installation.account_login,
            installation.created_by,
            installation.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn delete(&mut self, organization_id: OrganizationId) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "DELETE FROM github_installations WHERE organization_id = $1",
            organization_id.0,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}
