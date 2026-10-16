pub mod ports;

use chrono::{DateTime, Utc};

use crate::domain::agent::SecretToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegatedToken {
    pub token: SecretToken,
    pub expires_at: DateTime<Utc>,
}
