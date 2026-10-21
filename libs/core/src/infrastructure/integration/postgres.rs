use capitainerie_common::{ClientId, CoreError, IntegrationInstanceId, OrganizationId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        agent::SecretToken,
        integration::{
            BaseUrl, Environment, InstanceConnection, InstanceName, IntegrationInstance,
            IntegrationKind, ports::IntegrationInstanceRepository,
        },
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
};

const MISSING_KEY: &str = "SECRETS_KEY is not configured";

pub struct PgIntegrationInstanceRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgIntegrationInstanceRepository<'tx, 'c> {
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

impl PgIntegrationInstanceRepository<'_, '_> {
    pub(crate) async fn held_by_an_open_intervention(
        &self,
        organization_id: OrganizationId,
        id: IntegrationInstanceId,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM intervention_grants
                JOIN runs ON runs.id = intervention_grants.run_id
                WHERE runs.organization_id = $1
                  AND intervention_grants.instance_id = $2
                  AND runs.status IN ('queued', 'running')
            ) AS "held!"
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_one(&mut ***tx)
        .await
        .map_err(map_sqlx_error)
    }
}

fn binding(organization_id: OrganizationId, id: IntegrationInstanceId) -> Vec<u8> {
    format!("{organization_id}/integration_instance/{id}/secret").into_bytes()
}

fn write_error(error: sqlx::Error) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
            CoreError::Invalid("the client does not exist in this organization".into())
        }
        other => map_sqlx_error(other),
    }
}

impl IntegrationInstanceRepository for PgIntegrationInstanceRepository<'_, '_> {
    async fn insert(
        &mut self,
        instance: &IntegrationInstance,
        secret: &SecretToken,
    ) -> Result<(), CoreError> {
        let sealed = self.cipher()?.seal(
            secret.expose().as_bytes(),
            &binding(instance.organization_id, instance.id),
        )?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO integration_instances (
                id, organization_id, client_id, kind, name, environment, base_url, database, login,
                secret_nonce, secret_ciphertext, secret_preview, server_version, checked_at,
                created_by, created_at, updated_at, archived_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
            instance.id.0,
            instance.organization_id.0,
            instance.client_id.0,
            instance.kind.as_str(),
            instance.name.as_str(),
            instance.environment.as_str(),
            instance.base_url.as_str(),
            instance.database,
            instance.login,
            sealed.nonce,
            sealed.ciphertext,
            instance.secret_preview,
            instance.server_version,
            instance.checked_at,
            instance.created_by,
            instance.created_at,
            instance.updated_at,
            instance.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(write_error)
    }

    async fn update(
        &mut self,
        instance: &IntegrationInstance,
        secret: Option<&SecretToken>,
    ) -> Result<(), CoreError> {
        let sealed = secret
            .map(|secret| {
                self.cipher()?.seal(
                    secret.expose().as_bytes(),
                    &binding(instance.organization_id, instance.id),
                )
            })
            .transpose()?;
        let (nonce, ciphertext) = match sealed {
            Some(Sealed { nonce, ciphertext }) => (Some(nonce), Some(ciphertext)),
            None => (None, None),
        };
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE integration_instances
            SET name = $3, environment = $4, base_url = $5, database = $6, login = $7,
                secret_nonce = COALESCE($8, secret_nonce),
                secret_ciphertext = COALESCE($9, secret_ciphertext),
                secret_preview = $10, server_version = $11, checked_at = $12,
                updated_at = $13, archived_at = $14
            WHERE organization_id = $1 AND id = $2
            "#,
            instance.organization_id.0,
            instance.id.0,
            instance.name.as_str(),
            instance.environment.as_str(),
            instance.base_url.as_str(),
            instance.database,
            instance.login,
            nonce,
            ciphertext,
            instance.secret_preview,
            instance.server_version,
            instance.checked_at,
            instance.updated_at,
            instance.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(write_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationInstanceId,
    ) -> Result<Option<IntegrationInstance>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            InstanceRow,
            r#"
            SELECT id, organization_id, client_id, kind, name, environment, base_url, database,
                   login, secret_preview, server_version, checked_at, created_by, created_at,
                   updated_at, archived_at
            FROM integration_instances
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(IntegrationInstance::try_from)
        .transpose()
    }

    async fn connection(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationInstanceId,
    ) -> Result<Option<InstanceConnection>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT kind, base_url, database, login, secret_nonce, secret_ciphertext
            FROM integration_instances
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            let stored = |error: CoreError| {
                CoreError::Internal(format!("stored integration instance {id}: {error}"))
            };
            let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
            let plaintext = cipher.open(
                &Sealed {
                    nonce: row.secret_nonce,
                    ciphertext: row.secret_ciphertext,
                },
                &binding(organization_id, id),
            )?;
            let secret = String::from_utf8(plaintext).map_err(|_| {
                CoreError::Internal("a stored integration secret is not text".into())
            })?;
            Ok(InstanceConnection {
                instance_id: id,
                kind: IntegrationKind::try_from(row.kind.as_str()).map_err(stored)?,
                base_url: BaseUrl::try_from(row.base_url).map_err(stored)?,
                database: row.database,
                login: row.login,
                secret: SecretToken::new(secret),
            })
        })
        .transpose()
    }

    async fn list_for_client(
        &mut self,
        organization_id: OrganizationId,
        client_id: ClientId,
        include_archived: bool,
    ) -> Result<Vec<IntegrationInstance>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            InstanceRow,
            r#"
            SELECT id, organization_id, client_id, kind, name, environment, base_url, database,
                   login, secret_preview, server_version, checked_at, created_by, created_at,
                   updated_at, archived_at
            FROM integration_instances
            WHERE organization_id = $1 AND client_id = $2
              AND ($3 OR archived_at IS NULL)
            ORDER BY lower(name), id
            "#,
            organization_id.0,
            client_id.0,
            include_archived,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(IntegrationInstance::try_from)
        .collect()
    }
}

struct InstanceRow {
    id: Uuid,
    organization_id: Uuid,
    client_id: Uuid,
    kind: String,
    name: String,
    environment: String,
    base_url: String,
    database: Option<String>,
    login: Option<String>,
    secret_preview: String,
    server_version: Option<String>,
    checked_at: Option<DateTime<Utc>>,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<InstanceRow> for IntegrationInstance {
    type Error = CoreError;

    fn try_from(row: InstanceRow) -> Result<Self, Self::Error> {
        let stored = |error: CoreError| {
            CoreError::Internal(format!("stored integration instance {}: {error}", row.id))
        };
        Ok(Self {
            id: IntegrationInstanceId(row.id),
            organization_id: OrganizationId(row.organization_id),
            client_id: ClientId(row.client_id),
            kind: IntegrationKind::try_from(row.kind.as_str()).map_err(stored)?,
            name: InstanceName::try_from(row.name).map_err(stored)?,
            environment: Environment::try_from(row.environment.as_str()).map_err(stored)?,
            base_url: BaseUrl::try_from(row.base_url).map_err(stored)?,
            database: row.database,
            login: row.login,
            secret_preview: row.secret_preview,
            server_version: row.server_version,
            checked_at: row.checked_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}
