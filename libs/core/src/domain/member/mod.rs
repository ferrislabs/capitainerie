pub mod ports;
pub mod service;

use capitainerie_common::OrganizationId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub organization_id: OrganizationId,
    pub subject: String,
    pub name: String,
    pub email: Option<String>,
    pub last_seen_at: DateTime<Utc>,
}
