use capitainerie_common::{CoreError, OrganizationId, RunId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};

use crate::{
    domain::{
        agent::SecretToken,
        execution::{
            GithubAppCredentials, GithubAppSummary, GithubInstallation, TaskGrant,
            ports::{GithubAppRepository, GithubInstallationRepository, TaskGrantRepository},
        },
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
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
            INSERT INTO task_grants (token_sha256, run_id, organization_id, repository, branch, model, expires_at, revoked_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            token_sha256.as_slice(),
            grant.run_id.0,
            grant.organization_id.0,
            grant.repository.to_string(),
            grant.branch,
            grant.model,
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
            SELECT run_id, organization_id, repository, branch, model, expires_at, revoked_at
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
                model: row.model,
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

const MISSING_KEY: &str = "SECRETS_KEY is not configured";

pub struct PgGithubAppRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgGithubAppRepository<'tx, 'c> {
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

fn binding(organization_id: OrganizationId, field: &str) -> Vec<u8> {
    format!("{organization_id}/github_app/{field}").into_bytes()
}

fn open_secret(
    cipher: &SecretCipher,
    organization_id: OrganizationId,
    field: &str,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
) -> Result<SecretToken, CoreError> {
    let plaintext = cipher.open(
        &Sealed { nonce, ciphertext },
        &binding(organization_id, field),
    )?;
    String::from_utf8(plaintext)
        .map(SecretToken::new)
        .map_err(|_| CoreError::Internal(format!("a stored GitHub App {field} is not text")))
}

impl GithubAppRepository for PgGithubAppRepository<'_, '_> {
    async fn get(
        &mut self,
        organization_id: OrganizationId,
    ) -> Result<Option<GithubAppCredentials>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT app_id, slug, client_id, html_url, owner_login, owner_type,
                   client_secret_nonce, client_secret_ciphertext,
                   private_key_nonce, private_key_ciphertext,
                   webhook_secret_nonce, webhook_secret_ciphertext,
                   created_by, created_at
            FROM github_apps
            WHERE organization_id = $1
            "#,
            organization_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
            Ok(GithubAppCredentials {
                organization_id,
                app_id: row.app_id,
                slug: row.slug,
                client_id: row.client_id,
                client_secret: open_secret(
                    cipher,
                    organization_id,
                    "client_secret",
                    row.client_secret_nonce,
                    row.client_secret_ciphertext,
                )?,
                private_key_pem: open_secret(
                    cipher,
                    organization_id,
                    "private_key",
                    row.private_key_nonce,
                    row.private_key_ciphertext,
                )?,
                webhook_secret: open_secret(
                    cipher,
                    organization_id,
                    "webhook_secret",
                    row.webhook_secret_nonce,
                    row.webhook_secret_ciphertext,
                )?,
                html_url: row.html_url,
                owner_login: row.owner_login,
                owner_type: row.owner_type,
                created_by: row.created_by,
                created_at: row.created_at,
            })
        })
        .transpose()
    }

    async fn summary(
        &mut self,
        organization_id: OrganizationId,
    ) -> Result<Option<GithubAppSummary>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            SELECT app_id, slug, html_url, owner_login, owner_type, created_by, created_at
            FROM github_apps
            WHERE organization_id = $1
            "#,
            organization_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map(|row| {
            row.map(|row| GithubAppSummary {
                organization_id,
                app_id: row.app_id,
                slug: row.slug,
                html_url: row.html_url,
                owner_login: row.owner_login,
                owner_type: row.owner_type,
                created_by: row.created_by,
                created_at: row.created_at,
            })
        })
        .map_err(map_sqlx_error)
    }

    async fn insert(&mut self, app: &GithubAppCredentials) -> Result<(), CoreError> {
        let cipher = self.cipher()?;
        let id = app.organization_id;
        let client_secret = cipher.seal(
            app.client_secret.expose().as_bytes(),
            &binding(id, "client_secret"),
        )?;
        let private_key = cipher.seal(
            app.private_key_pem.expose().as_bytes(),
            &binding(id, "private_key"),
        )?;
        let webhook_secret = cipher.seal(
            app.webhook_secret.expose().as_bytes(),
            &binding(id, "webhook_secret"),
        )?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO github_apps (
                organization_id, app_id, slug, client_id, html_url, owner_login, owner_type,
                client_secret_nonce, client_secret_ciphertext,
                private_key_nonce, private_key_ciphertext,
                webhook_secret_nonce, webhook_secret_ciphertext,
                created_by, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            id.0,
            app.app_id,
            app.slug,
            app.client_id,
            app.html_url,
            app.owner_login,
            app.owner_type,
            client_secret.nonce,
            client_secret.ciphertext,
            private_key.nonce,
            private_key.ciphertext,
            webhook_secret.nonce,
            webhook_secret.ciphertext,
            app.created_by,
            app.created_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn delete(&mut self, organization_id: OrganizationId) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "DELETE FROM github_apps WHERE organization_id = $1",
            organization_id.0,
        )
        .execute(&mut ***tx)
        .await
        .map(|done| done.rows_affected() > 0)
        .map_err(map_sqlx_error)
    }
}
