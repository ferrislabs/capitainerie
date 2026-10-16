use capitainerie_common::{CoreError, RunId};
use chrono::{DateTime, Utc};

use crate::{
    domain::{
        agent::SecretToken,
        delegation::{DelegatedToken, ports::DelegationVault},
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
};

pub struct PgDelegationVault<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: &'c SecretCipher,
}

impl<'tx, 'c> PgDelegationVault<'tx, 'c> {
    pub fn new(tx: &SharedTx<'tx>, cipher: &'c SecretCipher) -> Self {
        Self {
            tx: tx.clone(),
            cipher,
        }
    }
}

fn binding(run_id: RunId, agent_client_id: &str) -> Vec<u8> {
    format!("{run_id}/{agent_client_id}").into_bytes()
}

impl DelegationVault for PgDelegationVault<'_, '_> {
    async fn store(
        &mut self,
        run_id: RunId,
        agent_client_id: &str,
        token: &DelegatedToken,
    ) -> Result<(), CoreError> {
        let sealed = self.cipher.seal(
            token.token.expose().as_bytes(),
            &binding(run_id, agent_client_id),
        )?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO run_delegations (run_id, agent_client_id, nonce, ciphertext, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (run_id, agent_client_id) DO UPDATE
            SET nonce = EXCLUDED.nonce, ciphertext = EXCLUDED.ciphertext, expires_at = EXCLUDED.expires_at
            "#,
            run_id.0,
            agent_client_id,
            sealed.nonce,
            sealed.ciphertext,
            token.expires_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn load(
        &mut self,
        run_id: RunId,
        agent_client_id: &str,
    ) -> Result<Option<DelegatedToken>, CoreError> {
        let row = {
            let mut tx = self.tx.lock().await;
            sqlx::query!(
                "SELECT nonce, ciphertext, expires_at FROM run_delegations WHERE run_id = $1 AND agent_client_id = $2",
                run_id.0,
                agent_client_id,
            )
            .fetch_optional(&mut ***tx)
            .await
            .map_err(map_sqlx_error)?
        };
        let Some(row) = row else { return Ok(None) };
        let plaintext = self.cipher.open(
            &Sealed {
                nonce: row.nonce,
                ciphertext: row.ciphertext,
            },
            &binding(run_id, agent_client_id),
        )?;
        let token = String::from_utf8(plaintext)
            .map_err(|_| CoreError::Internal("a stored delegation is not text".into()))?;
        Ok(Some(DelegatedToken {
            token: SecretToken::new(token),
            expires_at: row.expires_at,
        }))
    }

    async fn purge(&mut self, run_id: RunId) -> Result<(), CoreError> {
        purge_delegations(&self.tx, run_id).await
    }
}

/// Needs no key: destroying a delegation must work even when none can be read.
pub async fn purge_delegations(tx: &SharedTx<'_>, run_id: RunId) -> Result<(), CoreError> {
    let mut tx = tx.lock().await;
    sqlx::query!("DELETE FROM run_delegations WHERE run_id = $1", run_id.0)
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
}

pub fn is_expired(token: &DelegatedToken, now: DateTime<Utc>) -> bool {
    token.expires_at <= now
}
