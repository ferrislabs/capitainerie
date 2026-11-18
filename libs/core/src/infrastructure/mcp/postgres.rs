use capitainerie_common::{CoreError, McpConnectorId, OrganizationId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        agent::SecretToken,
        crew::Slug,
        mcp::{
            McpConnection, McpConnector, PendingAuthorization, TokenSet,
            ports::{McpAuthorizationRepository, McpConnectionRepository, McpConnectorRepository},
        },
    },
    infrastructure::{
        crypto::{Sealed, SecretCipher},
        postgres::{SharedTx, map_sqlx_error},
    },
};

const LIST_LIMIT: i64 = 500;
const MISSING_KEY: &str = "SECRETS_KEY is not configured";

#[repository(domain = McpConnector, backend = Postgres)]
pub struct PgMcpConnectorRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgMcpConnectorRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl McpConnectorRepository for PgMcpConnectorRepository<'_> {
    async fn insert(&mut self, connector: &McpConnector) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO mcp_connectors (
                id, organization_id, slug, name, url, resource, issuer, authorization_endpoint,
                token_endpoint, client_id, scopes, archived, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            connector.id.0,
            connector.organization_id.0,
            connector.slug.as_str(),
            connector.name,
            connector.url,
            connector.resource,
            connector.issuer,
            connector.authorization_endpoint,
            connector.token_endpoint,
            connector.client_id,
            &connector.scopes,
            connector.archived,
            connector.created_by,
            connector.created_at,
            connector.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| named(error, connector))
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: McpConnectorId,
    ) -> Result<Option<McpConnector>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ConnectorRow,
            r#"
            SELECT id, organization_id, slug, name, url, resource, issuer, authorization_endpoint,
                   token_endpoint, client_id, scopes, archived, created_by, created_at, updated_at
            FROM mcp_connectors
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(McpConnector::try_from)
        .transpose()
    }

    async fn update(&mut self, connector: &McpConnector) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE mcp_connectors
            SET name = $3, archived = $4, updated_at = $5
            WHERE organization_id = $1 AND id = $2
            "#,
            connector.organization_id.0,
            connector.id.0,
            connector.name,
            connector.archived,
            connector.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> Result<Vec<McpConnector>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ConnectorRow,
            r#"
            SELECT id, organization_id, slug, name, url, resource, issuer, authorization_endpoint,
                   token_endpoint, client_id, scopes, archived, created_by, created_at, updated_at
            FROM mcp_connectors
            WHERE organization_id = $1 AND ($2::bool IS NULL OR archived = $2)
            ORDER BY lower(name), id
            LIMIT $3
            "#,
            organization_id.0,
            archived,
            LIST_LIMIT,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(McpConnector::try_from)
        .collect()
    }
}

fn named(error: sqlx::Error, connector: &McpConnector) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.is_unique_violation() => CoreError::Conflict(format!(
            "un connecteur de slug {} existe déjà",
            connector.slug.as_str()
        )),
        other => map_sqlx_error(other),
    }
}

struct ConnectorRow {
    id: Uuid,
    organization_id: Uuid,
    slug: String,
    name: String,
    url: String,
    resource: String,
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    client_id: String,
    scopes: Vec<String>,
    archived: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ConnectorRow> for McpConnector {
    type Error = CoreError;

    fn try_from(row: ConnectorRow) -> Result<Self, Self::Error> {
        let stored = |error: CoreError| {
            CoreError::Internal(format!("stored MCP connector {}: {error}", row.id))
        };
        Ok(Self {
            id: McpConnectorId(row.id),
            organization_id: OrganizationId(row.organization_id),
            slug: Slug::try_from(row.slug).map_err(stored)?,
            name: row.name,
            url: row.url,
            resource: row.resource,
            issuer: row.issuer,
            authorization_endpoint: row.authorization_endpoint,
            token_endpoint: row.token_endpoint,
            client_id: row.client_id,
            scopes: row.scopes,
            archived: row.archived,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub struct PgMcpConnectionRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgMcpConnectionRepository<'tx, 'c> {
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

fn token_binding(
    organization_id: OrganizationId,
    connector_id: McpConnectorId,
    subject: &str,
    kind: &str,
) -> Vec<u8> {
    format!("mcp-connection/{organization_id}/{connector_id}/{subject}/{kind}").into_bytes()
}

fn state_binding(
    organization_id: OrganizationId,
    connector_id: McpConnectorId,
    subject: &str,
    state: &str,
) -> Vec<u8> {
    format!("mcp-state/{organization_id}/{connector_id}/{subject}/{state}").into_bytes()
}

fn open_text(cipher: &SecretCipher, sealed: &Sealed, binding: &[u8]) -> Result<String, CoreError> {
    String::from_utf8(cipher.open(sealed, binding)?)
        .map_err(|_| CoreError::Internal("a stored MCP secret is not text".into()))
}

impl McpConnectionRepository for PgMcpConnectionRepository<'_, '_> {
    async fn find(
        &mut self,
        organization_id: OrganizationId,
        connector_id: McpConnectorId,
        subject: &str,
    ) -> Result<Option<McpConnection>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            SELECT c.access_nonce, c.access_ciphertext, c.refresh_nonce, c.refresh_ciphertext,
                   c.expires_at, c.scope, c.created_at, c.updated_at
            FROM mcp_connections c
            JOIN mcp_connectors k ON k.id = c.connector_id
            WHERE k.organization_id = $1 AND c.connector_id = $2 AND c.subject = $3
            "#,
            organization_id.0,
            connector_id.0,
            subject,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
            let access = open_text(
                cipher,
                &Sealed {
                    nonce: row.access_nonce,
                    ciphertext: row.access_ciphertext,
                },
                &token_binding(organization_id, connector_id, subject, "access"),
            )?;
            let refresh = match (row.refresh_nonce, row.refresh_ciphertext) {
                (Some(nonce), Some(ciphertext)) => Some(SecretToken::new(open_text(
                    cipher,
                    &Sealed { nonce, ciphertext },
                    &token_binding(organization_id, connector_id, subject, "refresh"),
                )?)),
                _ => None,
            };
            Ok(McpConnection {
                connector_id,
                subject: subject.to_owned(),
                tokens: TokenSet {
                    access_token: SecretToken::new(access),
                    refresh_token: refresh,
                    expires_at: row.expires_at,
                    scope: row.scope,
                },
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .transpose()
    }

    async fn upsert(
        &mut self,
        organization_id: OrganizationId,
        connection: &McpConnection,
    ) -> Result<(), CoreError> {
        let cipher = self.cipher()?;
        let connector_id = connection.connector_id;
        let subject = connection.subject.as_str();
        let access = cipher.seal(
            connection.tokens.access_token.expose().as_bytes(),
            &token_binding(organization_id, connector_id, subject, "access"),
        )?;
        let refresh = connection
            .tokens
            .refresh_token
            .as_ref()
            .map(|token| {
                cipher.seal(
                    token.expose().as_bytes(),
                    &token_binding(organization_id, connector_id, subject, "refresh"),
                )
            })
            .transpose()?;
        let (refresh_nonce, refresh_ciphertext) = match refresh {
            Some(sealed) => (Some(sealed.nonce), Some(sealed.ciphertext)),
            None => (None, None),
        };
        let mut tx = self.tx.lock().await;
        let stored = sqlx::query!(
            r#"
            INSERT INTO mcp_connections (
                connector_id, subject, access_nonce, access_ciphertext, refresh_nonce,
                refresh_ciphertext, expires_at, scope, created_at, updated_at
            )
            SELECT $1, $3, $4, $5, $6, $7, $8, $9, $10, $11
            WHERE EXISTS (SELECT 1 FROM mcp_connectors WHERE id = $1 AND organization_id = $2)
            ON CONFLICT (connector_id, subject) DO UPDATE
            SET access_nonce = EXCLUDED.access_nonce,
                access_ciphertext = EXCLUDED.access_ciphertext,
                refresh_nonce = EXCLUDED.refresh_nonce,
                refresh_ciphertext = EXCLUDED.refresh_ciphertext,
                expires_at = EXCLUDED.expires_at,
                scope = EXCLUDED.scope,
                updated_at = EXCLUDED.updated_at
            "#,
            connector_id.0,
            organization_id.0,
            subject,
            access.nonce,
            access.ciphertext,
            refresh_nonce,
            refresh_ciphertext,
            connection.tokens.expires_at,
            connection.tokens.scope,
            connection.created_at,
            connection.updated_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        if stored.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn delete(
        &mut self,
        organization_id: OrganizationId,
        connector_id: McpConnectorId,
        subject: &str,
    ) -> Result<bool, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            DELETE FROM mcp_connections c
            USING mcp_connectors k
            WHERE k.id = c.connector_id AND k.organization_id = $1
              AND c.connector_id = $2 AND c.subject = $3
            "#,
            organization_id.0,
            connector_id.0,
            subject,
        )
        .execute(&mut ***tx)
        .await
        .map(|done| done.rows_affected() > 0)
        .map_err(map_sqlx_error)
    }

    async fn expiries(
        &mut self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> Result<Vec<(McpConnectorId, Option<DateTime<Utc>>)>, CoreError> {
        let mut tx = self.tx.lock().await;
        let rows = sqlx::query!(
            r#"
            SELECT c.connector_id, c.expires_at
            FROM mcp_connections c
            JOIN mcp_connectors k ON k.id = c.connector_id
            WHERE k.organization_id = $1 AND c.subject = $2
            "#,
            organization_id.0,
            subject,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows
            .into_iter()
            .map(|row| (McpConnectorId(row.connector_id), row.expires_at))
            .collect())
    }
}

pub struct PgMcpAuthorizationRepository<'tx, 'c> {
    tx: SharedTx<'tx>,
    cipher: Option<&'c SecretCipher>,
}

impl<'tx, 'c> PgMcpAuthorizationRepository<'tx, 'c> {
    pub fn new(tx: &SharedTx<'tx>, cipher: Option<&'c SecretCipher>) -> Self {
        Self {
            tx: tx.clone(),
            cipher,
        }
    }
}

impl McpAuthorizationRepository for PgMcpAuthorizationRepository<'_, '_> {
    async fn insert(
        &mut self,
        organization_id: OrganizationId,
        pending: &PendingAuthorization,
    ) -> Result<(), CoreError> {
        let cipher = self
            .cipher
            .ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
        let sealed = cipher.seal(
            pending.code_verifier.expose().as_bytes(),
            &state_binding(
                organization_id,
                pending.connector_id,
                &pending.subject,
                &pending.state,
            ),
        )?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO mcp_oauth_states
                (state, connector_id, subject, verifier_nonce, verifier_ciphertext, redirect_uri, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            pending.state,
            pending.connector_id.0,
            pending.subject,
            sealed.nonce,
            sealed.ciphertext,
            pending.redirect_uri,
            pending.expires_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn take(
        &mut self,
        organization_id: OrganizationId,
        state: &str,
    ) -> Result<Option<PendingAuthorization>, CoreError> {
        let cipher = self.cipher;
        let mut tx = self.tx.lock().await;
        let row = sqlx::query!(
            r#"
            DELETE FROM mcp_oauth_states s
            USING mcp_connectors k
            WHERE s.state = $1 AND k.id = s.connector_id AND k.organization_id = $2
            RETURNING s.connector_id AS "connector_id!", s.subject AS "subject!",
                      s.verifier_nonce AS "verifier_nonce!",
                      s.verifier_ciphertext AS "verifier_ciphertext!",
                      s.redirect_uri AS "redirect_uri!", s.expires_at AS "expires_at!"
            "#,
            state,
            organization_id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| {
            let cipher = cipher.ok_or_else(|| CoreError::Internal(MISSING_KEY.into()))?;
            let connector_id = McpConnectorId(row.connector_id);
            let verifier = open_text(
                cipher,
                &Sealed {
                    nonce: row.verifier_nonce,
                    ciphertext: row.verifier_ciphertext,
                },
                &state_binding(organization_id, connector_id, &row.subject, state),
            )?;
            Ok(PendingAuthorization {
                state: state.to_owned(),
                connector_id,
                subject: row.subject,
                code_verifier: SecretToken::new(verifier),
                redirect_uri: row.redirect_uri,
                expires_at: row.expires_at,
            })
        })
        .transpose()
    }

    async fn purge_expired(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!("DELETE FROM mcp_oauth_states WHERE expires_at <= $1", now)
            .execute(&mut ***tx)
            .await
            .map(|_| ())
            .map_err(map_sqlx_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_is_bound_to_its_organization_connector_and_subject() {
        let (org, other_org) = (OrganizationId::new(), OrganizationId::new());
        let connector = McpConnectorId::new();
        let base = token_binding(org, connector, "user-1", "access");

        assert_ne!(
            base,
            token_binding(other_org, connector, "user-1", "access")
        );
        assert_ne!(
            base,
            token_binding(org, McpConnectorId::new(), "user-1", "access")
        );
        assert_ne!(base, token_binding(org, connector, "user-2", "access"));
        assert_ne!(base, token_binding(org, connector, "user-1", "refresh"));
        assert_ne!(
            state_binding(org, connector, "user-1", "a"),
            state_binding(org, connector, "user-1", "b")
        );
    }

    #[test]
    fn a_sealed_token_does_not_open_for_another_subject() {
        let cipher = SecretCipher::new(&[9u8; 32]).unwrap();
        let (org, connector) = (OrganizationId::new(), McpConnectorId::new());
        let sealed = cipher
            .seal(b"token", &token_binding(org, connector, "user-1", "access"))
            .unwrap();

        assert!(
            open_text(
                &cipher,
                &sealed,
                &token_binding(org, connector, "user-2", "access")
            )
            .is_err()
        );
        assert_eq!(
            open_text(
                &cipher,
                &sealed,
                &token_binding(org, connector, "user-1", "access")
            )
            .unwrap(),
            "token"
        );
    }
}
