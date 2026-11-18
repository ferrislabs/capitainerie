use capitainerie_common::{CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use crate::{
    domain::{
        agent::SecretToken,
        provider::ProviderKind,
        user_provider::{
            UserProviderCredential, UserProviderSummary, ports::UserProviderRepository,
        },
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
};

const MISSING_KEY: &str = "SECRETS_KEY is not configured";

pub struct PgUserProviderRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgUserProviderRepository<'tx, 'c> {
    pub fn new(tx: &SharedTx<'tx>, cipher: Option<&'c SecretCipher>) -> Self {
        Self {
            tx: tx.clone(),
            cipher,
        }
    }
}

fn binding(organization_id: OrganizationId, subject: &str, kind: ProviderKind) -> Vec<u8> {
    format!("user/{organization_id}/{subject}/{}", kind.as_str()).into_bytes()
}

struct Row {
    kind: String,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    key_preview: String,
    base_url: Option<String>,
    default_model: String,
    is_default: bool,
    updated_at: DateTime<Utc>,
}

fn unseal(
    cipher: Option<&SecretCipher>,
    organization_id: OrganizationId,
    subject: &str,
    row: Row,
) -> Result<UserProviderCredential, CoreError> {
    let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
    let kind = ProviderKind::try_from(row.kind.as_str())?;
    let plaintext = cipher.open(
        &Sealed {
            nonce: row.nonce,
            ciphertext: row.ciphertext,
        },
        &binding(organization_id, subject, kind),
    )?;
    let secret = String::from_utf8(plaintext)
        .map_err(|_| CoreError::Internal("a stored provider key is not text".into()))?;
    Ok(UserProviderCredential {
        organization_id,
        subject: subject.to_owned(),
        kind,
        secret: SecretToken::new(secret),
        base_url: row.base_url,
        default_model: row.default_model,
        preview: row.key_preview,
        is_default: row.is_default,
        updated_at: row.updated_at,
    })
}

impl UserProviderRepository for PgUserProviderRepository<'_, '_> {
    async fn upsert(&mut self, credential: &UserProviderCredential) -> Result<(), CoreError> {
        let cipher = self
            .cipher
            .ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
        let sealed = cipher.seal(
            credential.secret.expose().as_bytes(),
            &binding(
                credential.organization_id,
                &credential.subject,
                credential.kind,
            ),
        )?;
        let mut tx = self.tx.lock().await;
        if credential.is_default {
            sqlx::query!(
                r#"
                UPDATE user_provider_credentials SET is_default = FALSE
                WHERE organization_id = $1 AND subject = $2 AND kind <> $3 AND is_default
                "#,
                credential.organization_id.0,
                credential.subject,
                credential.kind.as_str(),
            )
            .execute(&mut ***tx)
            .await
            .map_err(map_sqlx_error)?;
        }
        sqlx::query!(
            r#"
            INSERT INTO user_provider_credentials
                (organization_id, subject, kind, nonce, ciphertext, key_preview, base_url,
                 default_model, is_default, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
            ON CONFLICT (organization_id, subject, kind) DO UPDATE
            SET nonce = EXCLUDED.nonce, ciphertext = EXCLUDED.ciphertext,
                key_preview = EXCLUDED.key_preview, base_url = EXCLUDED.base_url,
                default_model = EXCLUDED.default_model, is_default = EXCLUDED.is_default,
                updated_at = EXCLUDED.updated_at
            "#,
            credential.organization_id.0,
            credential.subject,
            credential.kind.as_str(),
            sealed.nonce,
            sealed.ciphertext,
            credential.preview,
            credential.base_url,
            credential.default_model,
            credential.is_default,
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
        subject: &str,
    ) -> Result<Vec<UserProviderSummary>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT kind, base_url, default_model, key_preview, is_default, updated_at
            FROM user_provider_credentials
            WHERE organization_id = $1 AND subject = $2
            ORDER BY kind
            "#,
            organization_id.0,
            subject,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| {
                Ok(UserProviderSummary {
                    kind: ProviderKind::try_from(row.kind.as_str())?,
                    base_url: row.base_url,
                    default_model: row.default_model,
                    preview: row.key_preview,
                    is_default: row.is_default,
                    updated_at: row.updated_at,
                })
            })
            .collect()
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
        kind: ProviderKind,
    ) -> Result<Option<UserProviderCredential>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let row = sqlx::query_as!(
            Row,
            r#"
            SELECT kind, nonce, ciphertext, key_preview, base_url, default_model, is_default, updated_at
            FROM user_provider_credentials
            WHERE organization_id = $1 AND subject = $2 AND kind = $3
            "#,
            organization_id.0,
            subject,
            kind.as_str(),
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| unseal(cipher, organization_id, subject, row))
            .transpose()
    }

    async fn all(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> Result<Vec<UserProviderCredential>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query_as!(
            Row,
            r#"
            SELECT kind, nonce, ciphertext, key_preview, base_url, default_model, is_default, updated_at
            FROM user_provider_credentials
            WHERE organization_id = $1 AND subject = $2
            ORDER BY kind
            "#,
            organization_id.0,
            subject,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| unseal(cipher, organization_id, subject, row))
            .collect()
    }

    async fn delete(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
        kind: ProviderKind,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            "DELETE FROM user_provider_credentials WHERE organization_id = $1 AND subject = $2 AND kind = $3",
            organization_id.0,
            subject,
            kind.as_str(),
        )
        .execute(&mut ***tx)
        .await
        .map(|done| done.rows_affected() > 0)
        .map_err(map_sqlx_error)
    }

    async fn set_default(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
        kind: ProviderKind,
    ) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            UPDATE user_provider_credentials SET is_default = FALSE
            WHERE organization_id = $1 AND subject = $2 AND kind <> $3 AND is_default
            "#,
            organization_id.0,
            subject,
            kind.as_str(),
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        sqlx::query!(
            r#"
            UPDATE user_provider_credentials SET is_default = TRUE
            WHERE organization_id = $1 AND subject = $2 AND kind = $3
            "#,
            organization_id.0,
            subject,
            kind.as_str(),
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }
}
