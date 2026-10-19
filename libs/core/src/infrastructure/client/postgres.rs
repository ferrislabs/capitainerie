use capitainerie_common::{ClientId, CoreError, OrganizationId};
use capitainerie_macros::repository;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        card::Column,
        client::{Client, ClientListing, ColumnCount, ListFilter, ports::ClientRepository},
        label::{Aliases, Name},
    },
    infrastructure::postgres::{SharedTx, map_sqlx_error},
};

const LIST_LIMIT: i64 = 500;

#[repository(domain = Client, backend = Postgres)]
pub struct PgClientRepository<'tx> {
    tx: SharedTx<'tx>,
}

impl<'tx> PgClientRepository<'tx> {
    pub fn new(tx: &SharedTx<'tx>) -> Self {
        Self { tx: tx.clone() }
    }
}

impl ClientRepository for PgClientRepository<'_> {
    async fn insert(&mut self, client: &Client) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            INSERT INTO clients (
                id, organization_id, name, aliases, notes, created_by, created_at, updated_at, archived_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            client.id.0,
            client.organization_id.0,
            client.name.as_str(),
            client.aliases.as_slice(),
            client.notes,
            client.created_by,
            client.created_at,
            client.updated_at,
            client.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map(|_| ())
        .map_err(|error| named(error, client))
    }

    async fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ClientId,
    ) -> Result<Option<Client>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ClientRow,
            r#"
            SELECT id, organization_id, name, aliases, notes, created_by, created_at, updated_at, archived_at
            FROM clients
            WHERE organization_id = $1 AND id = $2
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_optional(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Client::try_from)
        .transpose()
    }

    async fn update(&mut self, client: &Client) -> Result<(), CoreError> {
        let mut tx = self.tx.lock().await;
        let updated = sqlx::query!(
            r#"
            UPDATE clients
            SET name = $3, aliases = $4, notes = $5, updated_at = $6, archived_at = $7
            WHERE organization_id = $1 AND id = $2
            "#,
            client.organization_id.0,
            client.id.0,
            client.name.as_str(),
            client.aliases.as_slice(),
            client.notes,
            client.updated_at,
            client.archived_at,
        )
        .execute(&mut ***tx)
        .await
        .map_err(|error| named(error, client))?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    async fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &ListFilter,
    ) -> Result<Vec<ClientListing>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            ListingRow,
            r#"
            SELECT c.id, c.organization_id, c.name, c.aliases, c.notes, c.created_by,
                   c.created_at, c.updated_at, c.archived_at,
                   (SELECT count(*) FROM projects p
                     WHERE p.organization_id = c.organization_id AND p.client_id = c.id
                       AND p.archived_at IS NULL) AS "projects!",
                   (SELECT count(*) FROM cards k
                     WHERE k.organization_id = c.organization_id AND k.client_id = c.id
                       AND k."column" IN ('proposed', 'todo', 'in_progress')) AS "open_cards!"
            FROM clients c
            WHERE c.organization_id = $1
              AND (c.archived_at IS NOT NULL) = $2
              AND ($3::text IS NULL
                   OR strpos(lower(c.name), lower($3)) > 0
                   OR EXISTS (SELECT 1 FROM unnest(c.aliases) AS alias WHERE strpos(lower(alias), lower($3)) > 0))
            ORDER BY lower(c.name), c.id
            LIMIT $4
            "#,
            organization_id.0,
            filter.only_archived(),
            filter.needle(),
            LIST_LIMIT,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(ClientListing::try_from)
        .collect()
    }

    async fn card_counts(
        &mut self,
        organization_id: OrganizationId,
        id: ClientId,
    ) -> Result<Vec<ColumnCount>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"
            SELECT "column" AS "column!", count(*) AS "cards!"
            FROM cards
            WHERE organization_id = $1 AND client_id = $2
            GROUP BY "column"
            "#,
            organization_id.0,
            id.0,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(|row| {
            Ok(ColumnCount {
                column: Column::try_from(row.column.as_str())?,
                cards: u64::try_from(row.cards).unwrap_or_default(),
            })
        })
        .collect()
    }

    async fn recent(
        &mut self,
        organization_id: OrganizationId,
        limit: i64,
    ) -> Result<Vec<(DateTime<Utc>, Client)>, CoreError> {
        let mut tx = self.tx.lock().await;
        sqlx::query_as!(
            RecentRow,
            r#"
            SELECT c.id, c.organization_id, c.name, c.aliases, c.notes, c.created_by,
                   c.created_at, c.updated_at, c.archived_at,
                   GREATEST(
                       c.updated_at,
                       COALESCE((SELECT max(k.updated_at) FROM cards k
                                  WHERE k.organization_id = c.organization_id AND k.client_id = c.id), c.updated_at),
                       COALESCE((SELECT max(t.created_at) FROM transcripts t
                                  WHERE t.organization_id = c.organization_id AND t.client_id = c.id), c.updated_at)
                   ) AS "used_at!"
            FROM clients c
            WHERE c.organization_id = $1 AND c.archived_at IS NULL
            ORDER BY "used_at!" DESC, c.id
            LIMIT $2
            "#,
            organization_id.0,
            limit,
        )
        .fetch_all(&mut ***tx)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(<(DateTime<Utc>, Client)>::try_from)
        .collect()
    }
}

fn named(error: sqlx::Error, client: &Client) -> CoreError {
    match error {
        sqlx::Error::Database(db) if db.is_unique_violation() => CoreError::Conflict(format!(
            "a client named {} already exists",
            client.name.as_str()
        )),
        other => map_sqlx_error(other),
    }
}

struct ClientRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    aliases: Vec<String>,
    notes: String,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<ClientRow> for Client {
    type Error = CoreError;

    fn try_from(row: ClientRow) -> Result<Self, Self::Error> {
        let stored =
            |error: CoreError| CoreError::Internal(format!("stored client {}: {error}", row.id));
        Ok(Self {
            id: ClientId(row.id),
            organization_id: OrganizationId(row.organization_id),
            name: Name::try_from(row.name).map_err(stored)?,
            aliases: Aliases::try_from(row.aliases).map_err(stored)?,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
    }
}

struct ListingRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    aliases: Vec<String>,
    notes: String,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    projects: i64,
    open_cards: i64,
}

impl TryFrom<ListingRow> for ClientListing {
    type Error = CoreError;

    fn try_from(row: ListingRow) -> Result<Self, Self::Error> {
        let projects = u64::try_from(row.projects).unwrap_or_default();
        let open_cards = u64::try_from(row.open_cards).unwrap_or_default();
        Ok(Self {
            client: Client::try_from(ClientRow {
                id: row.id,
                organization_id: row.organization_id,
                name: row.name,
                aliases: row.aliases,
                notes: row.notes,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
                archived_at: row.archived_at,
            })?,
            projects,
            open_cards,
        })
    }
}

struct RecentRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    aliases: Vec<String>,
    notes: String,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    used_at: DateTime<Utc>,
}

impl TryFrom<RecentRow> for (DateTime<Utc>, Client) {
    type Error = CoreError;

    fn try_from(row: RecentRow) -> Result<Self, Self::Error> {
        let used_at = row.used_at;
        Client::try_from(ClientRow {
            id: row.id,
            organization_id: row.organization_id,
            name: row.name,
            aliases: row.aliases,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        })
        .map(|client| (used_at, client))
    }
}
