use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_macros::repository;

use crate::{
    domain::{
        agent::SecretToken,
        provider::{
            ModelAssignments, ProviderCredential, ProviderKind, ProviderSummary,
            ports::{ModelAssignmentRepository, ProviderRepository},
        },
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
};

const MISSING_KEY: &str = "SECRETS_KEY is not configured";

pub struct PgProviderRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgProviderRepository<'tx, 'c> {
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

fn binding(organization_id: OrganizationId, kind: ProviderKind) -> Vec<u8> {
    format!("{organization_id}/{}", kind.as_str()).into_bytes()
}

impl ProviderRepository for PgProviderRepository<'_, '_> {
    async fn upsert(&mut self, credential: &ProviderCredential) -> Result<(), CoreError> {
        let sealed = self.cipher()?.seal(
            credential.secret.expose().as_bytes(),
            &binding(credential.organization_id, credential.kind),
        )?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO provider_credentials
                (organization_id, kind, nonce, ciphertext, base_url, preview, created_by, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (organization_id, kind) DO UPDATE
            SET nonce = EXCLUDED.nonce, ciphertext = EXCLUDED.ciphertext,
                base_url = EXCLUDED.base_url, preview = EXCLUDED.preview,
                created_by = EXCLUDED.created_by, updated_at = EXCLUDED.updated_at
            "#,
            credential.organization_id.0,
            credential.kind.as_str(),
            sealed.nonce,
            sealed.ciphertext,
            credential.base_url,
            credential.preview,
            credential.created_by,
            credential.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
    ) -> Result<Vec<ProviderSummary>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT kind, base_url, preview, created_by, updated_at
            FROM provider_credentials
            WHERE organization_id = $1
            ORDER BY kind
            "#,
            organization_id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| {
                Ok(ProviderSummary {
                    kind: ProviderKind::try_from(row.kind.as_str())?,
                    base_url: row.base_url,
                    preview: row.preview,
                    created_by: row.created_by,
                    updated_at: row.updated_at,
                })
            })
            .collect()
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        kind: ProviderKind,
    ) -> Result<Option<ProviderCredential>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT nonce, ciphertext, base_url, preview, created_by, updated_at
            FROM provider_credentials
            WHERE organization_id = $1 AND kind = $2
            "#,
            organization_id.0,
            kind.as_str(),
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
            let plaintext = cipher.open(
                &Sealed {
                    nonce: row.nonce,
                    ciphertext: row.ciphertext,
                },
                &binding(organization_id, kind),
            )?;
            let secret = String::from_utf8(plaintext)
                .map_err(|_| CoreError::Internal("a stored provider key is not text".into()))?;
            Ok(ProviderCredential {
                organization_id,
                kind,
                secret: SecretToken::new(secret),
                base_url: row.base_url,
                preview: row.preview,
                created_by: row.created_by,
                updated_at: row.updated_at,
            })
        })
        .transpose()
    }

    async fn delete(
        &mut self,
        organization_id: OrganizationId,
        kind: ProviderKind,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "DELETE FROM provider_credentials WHERE organization_id = $1 AND kind = $2",
            organization_id.0,
            kind.as_str(),
        )
        .execute(&mut ***tx)
        .await
        .map(|done| done.rows_affected() > 0)
        .map_err(map_sqlx_error)
    }
}

#[repository(domain = ModelAssignment, backend = Postgres)]
pub struct PgModelAssignmentRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgModelAssignmentRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl ModelAssignmentRepository for PgModelAssignmentRepository<'_> {
    async fn get(
        &mut self,
        organization_id: OrganizationId,
    ) -> Result<Option<ModelAssignments>, CoreError> {
        let mut tx = self.tx.lock().await;
        let stored = sqlx::query_scalar!(
            "SELECT assignments FROM model_assignments WHERE organization_id = $1",
            organization_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        stored
            .map(|value| {
                serde_json::from_value(value).map_err(|error| {
                    CoreError::Internal(format!("stored model assignments: {error}"))
                })
            })
            .transpose()
    }

    async fn put(
        &mut self,
        organization_id: OrganizationId,
        assignments: &ModelAssignments,
        updated_by: &str,
    ) -> Result<(), CoreError> {
        let value = serde_json::to_value(assignments)
            .map_err(|error| CoreError::Internal(format!("encoding model assignments: {error}")))?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO model_assignments (organization_id, assignments, updated_by, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (organization_id) DO UPDATE
            SET assignments = EXCLUDED.assignments, updated_by = EXCLUDED.updated_by,
                updated_at = EXCLUDED.updated_at
            "#,
            organization_id.0,
            value,
            updated_by,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}
