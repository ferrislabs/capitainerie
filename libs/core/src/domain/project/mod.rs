pub mod ports;
pub mod service;

use capitainerie_common::{ClientId, OrganizationId, ProjectId};
use chrono::{DateTime, Utc};

use crate::domain::{
    client::{ClientRef, ColumnCount},
    execution::RepositoryRef,
    label::{Aliases, Name},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: ProjectId,
    pub organization_id: OrganizationId,
    pub client: Option<ClientRef>,
    pub name: Name,
    pub aliases: Aliases,
    pub repository: Option<RepositoryRef>,
    pub description: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRef {
    pub id: ProjectId,
    pub name: String,
}

impl Project {
    pub fn reference(&self) -> ProjectRef {
        ProjectRef {
            id: self.id,
            name: self.name.as_str().to_owned(),
        }
    }

    pub fn client_id(&self) -> Option<ClientId> {
        self.client.as_ref().map(|client| client.id)
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectFilter {
    pub query: Option<String>,
    pub archived: Option<bool>,
    pub client_id: Option<ClientId>,
}

impl ProjectFilter {
    pub fn only_archived(&self) -> bool {
        self.archived == Some(true)
    }

    pub fn needle(&self) -> Option<&str> {
        self.query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListing {
    pub project: Project,
    pub open_cards: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDetail {
    pub project: Project,
    pub cards: Vec<ColumnCount>,
}
