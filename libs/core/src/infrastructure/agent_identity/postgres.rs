use capitainerie_common::{AgentId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        agent::SecretToken,
        agent_identity::{
            AgentClientId, AgentIdentity, CLIENT_PREFIX, IdentityState, IdentityStatus,
            IdentitySummary, ProvisionedClient, UnsyncedClient, ports::AgentIdentityRepository,
        },
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
};

const MISSING_KEY: &str = "SECRETS_KEY is not configured";

pub struct PgAgentIdentityRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgAgentIdentityRepository<'tx, 'c> {
    pub fn new(tx: &SharedTx<'tx>, cipher: Option<&'c SecretCipher>) -> Self {
        Self {
            tx: tx.clone(),
            cipher,
        }
    }
}

/// Binds a sealed secret to its agent and its FerrisKey client: a secret
/// copied onto another row fails to open.
fn secret_binding(
    organization_id: OrganizationId,
    agent_id: AgentId,
    client_uuid: &str,
) -> Vec<u8> {
    format!("agent-identity/{organization_id}/{agent_id}/{client_uuid}").into_bytes()
}

struct Row {
    organization_id: Uuid,
    agent_id: Uuid,
    client_uuid: Option<String>,
    status: String,
    secret_nonce: Option<Vec<u8>>,
    secret_ciphertext: Option<Vec<u8>>,
    failure: Option<String>,
    synced: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// The columns an identity is written to, its secret sealed.
struct Columns<'a> {
    client_uuid: Option<&'a str>,
    status: &'static str,
    secret_nonce: Option<Vec<u8>>,
    secret_ciphertext: Option<Vec<u8>>,
    failure: Option<&'a str>,
}

impl<'a> Columns<'a> {
    fn of(identity: &'a AgentIdentity, cipher: Option<&SecretCipher>) -> Result<Self, CoreError> {
        let (client_uuid, failure, client) = match &identity.state {
            IdentityState::Pending { failure } => (None, failure.as_deref(), None),
            IdentityState::Active(client) => {
                (Some(client.client_uuid.as_str()), None, Some(client))
            }
            IdentityState::Disabled { client } => (
                client.as_ref().map(|client| client.client_uuid.as_str()),
                None,
                client.as_ref(),
            ),
        };
        let sealed = client
            .map(|client| {
                let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
                cipher.seal(
                    client.secret.expose().as_bytes(),
                    &secret_binding(
                        identity.organization_id,
                        identity.agent_id,
                        &client.client_uuid,
                    ),
                )
            })
            .transpose()?;
        let (secret_nonce, secret_ciphertext) = match sealed {
            Some(sealed) => (Some(sealed.nonce), Some(sealed.ciphertext)),
            None => (None, None),
        };
        Ok(Self {
            client_uuid,
            status: identity.status().as_str(),
            secret_nonce,
            secret_ciphertext,
            failure,
        })
    }
}

impl Row {
    fn into_identity(self, cipher: Option<&SecretCipher>) -> Result<AgentIdentity, CoreError> {
        let organization_id = OrganizationId(self.organization_id);
        let agent_id = AgentId(self.agent_id);
        let client = match (self.client_uuid, self.secret_nonce, self.secret_ciphertext) {
            (Some(client_uuid), Some(nonce), Some(ciphertext)) => {
                let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
                let secret = cipher.open(
                    &Sealed { nonce, ciphertext },
                    &secret_binding(organization_id, agent_id, &client_uuid),
                )?;
                let secret = String::from_utf8(secret)
                    .map_err(|_| CoreError::Internal("a stored agent secret is not text".into()))?;
                Some(ProvisionedClient {
                    client_uuid,
                    secret: SecretToken::new(secret),
                })
            }
            _ => None,
        };
        let state = match (IdentityStatus::try_from(self.status.as_str())?, client) {
            (IdentityStatus::Pending, _) => IdentityState::Pending {
                failure: self.failure,
            },
            (IdentityStatus::Active, Some(client)) => IdentityState::Active(client),
            (IdentityStatus::Active, None) => {
                return Err(CoreError::Internal(
                    "an active agent identity has no client".into(),
                ));
            }
            (IdentityStatus::Disabled, client) => IdentityState::Disabled { client },
        };
        Ok(AgentIdentity {
            organization_id,
            agent_id,
            client_id: AgentClientId::of(agent_id),
            state,
            synced: self.synced,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

impl AgentIdentityRepository for PgAgentIdentityRepository<'_, '_> {
    async fn find(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
    ) -> Result<Option<AgentIdentity>, CoreError> {
        let mut tx = self.tx.lock().await;
        let row = sqlx::query_as!(
            Row,
            r#"
            SELECT organization_id, agent_id, client_uuid, status, secret_nonce,
                   secret_ciphertext, failure, synced, created_at, updated_at
            FROM agent_identities
            WHERE organization_id = $1 AND agent_id = $2
            "#,
            organization_id.0,
            agent_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| row.into_identity(self.cipher)).transpose()
    }

    async fn upsert(&mut self, identity: &AgentIdentity) -> Result<(), CoreError> {
        let columns = Columns::of(identity, self.cipher)?;
        let mut tx = self.tx.lock().await;
        let stored = sqlx::query!(
            r#"
            INSERT INTO agent_identities (
                agent_id, organization_id, client_id, client_uuid, status, secret_nonce,
                secret_ciphertext, failure, synced, created_at, updated_at
            )
            SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            WHERE EXISTS (SELECT 1 FROM agents WHERE id = $1 AND organization_id = $2)
            ON CONFLICT (agent_id) DO UPDATE
            SET client_uuid = EXCLUDED.client_uuid,
                status = EXCLUDED.status,
                secret_nonce = EXCLUDED.secret_nonce,
                secret_ciphertext = EXCLUDED.secret_ciphertext,
                failure = EXCLUDED.failure,
                synced = EXCLUDED.synced,
                updated_at = EXCLUDED.updated_at
            WHERE agent_identities.organization_id = EXCLUDED.organization_id
            "#,
            identity.agent_id.0,
            identity.organization_id.0,
            identity.client_id.as_str(),
            columns.client_uuid,
            columns.status,
            columns.secret_nonce,
            columns.secret_ciphertext,
            columns.failure,
            identity.synced,
            identity.created_at,
            identity.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if stored.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn upsert_unchanged(
        &mut self,
        identity: &AgentIdentity,
        expected: DateTime<Utc>,
    ) -> Result<bool, CoreError> {
        let columns = Columns::of(identity, self.cipher)?;
        let mut tx = self.tx.lock().await;
        let stored = sqlx::query!(
            r#"
            UPDATE agent_identities
            SET client_uuid = $3,
                status = $4,
                secret_nonce = $5,
                secret_ciphertext = $6,
                failure = $7,
                synced = $8,
                updated_at = $9
            WHERE organization_id = $1 AND agent_id = $2 AND updated_at = $10
            "#,
            identity.organization_id.0,
            identity.agent_id.0,
            columns.client_uuid,
            columns.status,
            columns.secret_nonce,
            columns.secret_ciphertext,
            columns.failure,
            identity.synced,
            identity.updated_at,
            expected,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(stored.rows_affected() == 1)
    }

    async fn summary(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
    ) -> Result<Option<IdentitySummary>, CoreError> {
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT status, failure, synced, updated_at
            FROM agent_identities
            WHERE organization_id = $1 AND agent_id = $2
            "#,
            organization_id.0,
            agent_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            let status = IdentityStatus::try_from(row.status.as_str())?;
            Ok(IdentitySummary {
                client_id: AgentClientId::of(agent_id),
                status,
                failure: row.failure.filter(|_| status == IdentityStatus::Pending),
                synced: row.synced,
                updated_at: row.updated_at,
            })
        })
        .transpose()
    }

    async fn pending(
        &mut self,
        retry_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<AgentIdentity>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query_as!(
            Row,
            r#"
            SELECT organization_id, agent_id, client_uuid, status, secret_nonce,
                   secret_ciphertext, failure, synced, created_at, updated_at
            FROM agent_identities
            WHERE status = 'pending' AND (failure IS NULL OR updated_at < $1)
            ORDER BY updated_at
            LIMIT $2
            "#,
            retry_before,
            limit,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter()
            .map(|row| row.into_identity(self.cipher))
            .collect()
    }

    async fn unsynced(&mut self, limit: i64) -> Result<Vec<UnsyncedClient>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT organization_id, agent_id, client_uuid AS "client_uuid!", status, updated_at
            FROM agent_identities
            WHERE NOT synced AND client_uuid IS NOT NULL AND status <> 'pending'
            ORDER BY updated_at
            LIMIT $1
            "#,
            limit,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| UnsyncedClient {
                organization_id: OrganizationId(row.organization_id),
                agent_id: AgentId(row.agent_id),
                client_uuid: row.client_uuid,
                enabled: row.status == IdentityStatus::Active.as_str(),
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn mark_synced(
        &mut self,
        organization_id: OrganizationId,
        agent_id: AgentId,
        expected: DateTime<Utc>,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        let marked = sqlx::query!(
            r#"
            UPDATE agent_identities
            SET synced = true
            WHERE organization_id = $1 AND agent_id = $2 AND updated_at = $3 AND NOT synced
            "#,
            organization_id.0,
            agent_id.0,
            expected,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(marked.rows_affected() == 1)
    }

    async fn adopt(&mut self, now: DateTime<Utc>) -> Result<u64, CoreError> {
        let mut tx = self.tx.lock().await;
        let adopted = sqlx::query!(
            r#"
            INSERT INTO agent_identities (
                agent_id, organization_id, client_id, status, synced, created_at, updated_at
            )
            SELECT agents.id, agents.organization_id, $1 || agents.id::text,
                   CASE WHEN agents.archived THEN 'disabled' ELSE 'pending' END,
                   true, $2, $2
            FROM agents
            WHERE NOT EXISTS (
                SELECT 1 FROM agent_identities WHERE agent_identities.agent_id = agents.id
            )
            ON CONFLICT (agent_id) DO NOTHING
            "#,
            CLIENT_PREFIX,
            now,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(adopted.rows_affected())
    }
}
