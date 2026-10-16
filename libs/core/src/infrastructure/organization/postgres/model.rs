use capitainerie_common::{CoreError, OrganizationId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{Organization, OrganizationSlug};

pub(super) struct OrganizationRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub issuer_url: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<OrganizationRow> for Organization {
    type Error = CoreError;

    fn try_from(row: OrganizationRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: OrganizationId(row.id),
            slug: OrganizationSlug::try_from(row.slug)
                .map_err(|error| CoreError::Internal(format!("stored slug: {error}")))?,
            name: row.name,
            issuer_url: row.issuer_url,
            created_at: row.created_at,
        })
    }
}
