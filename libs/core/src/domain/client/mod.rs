pub mod catalog;
pub mod links;
pub mod ports;
pub mod service;

use capitainerie_common::{ClientId, OrganizationId};
use chrono::{DateTime, Utc};

use crate::domain::{
    card::Column,
    label::{Aliases, Name},
    project::Project,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    pub id: ClientId,
    pub organization_id: OrganizationId,
    pub name: Name,
    pub aliases: Aliases,
    pub notes: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRef {
    pub id: ClientId,
    pub name: String,
}

impl Client {
    pub fn reference(&self) -> ClientRef {
        ClientRef {
            id: self.id,
            name: self.name.as_str().to_owned(),
        }
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListFilter {
    pub query: Option<String>,
    pub archived: Option<bool>,
}

impl ListFilter {
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
pub struct ClientListing {
    pub client: Client,
    pub projects: u64,
    pub open_cards: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnCount {
    pub column: Column,
    pub cards: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDetail {
    pub client: Client,
    pub projects: Vec<Project>,
    pub cards: Vec<ColumnCount>,
}

impl ColumnCount {
    pub fn complete(counted: &[ColumnCount]) -> Vec<ColumnCount> {
        Column::ALL
            .into_iter()
            .map(|column| ColumnCount {
                column,
                cards: counted
                    .iter()
                    .find(|count| count.column == column)
                    .map_or(0, |count| count.cards),
            })
            .collect()
    }
}
