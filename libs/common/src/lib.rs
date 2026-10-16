use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("resource not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("forbidden{}", .reason.as_ref().map(|r| format!(": {r}")).unwrap_or_default())]
    Forbidden { reason: Option<String> },

    #[error("database error: {0}")]
    Database(String),

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct OrganizationId(pub Uuid);

impl OrganizationId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for OrganizationId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_without_reason_has_no_suffix() {
        assert_eq!(
            CoreError::Forbidden { reason: None }.to_string(),
            "forbidden"
        );
    }

    #[test]
    fn forbidden_with_reason_appends_it() {
        let error = CoreError::Forbidden {
            reason: Some("not a member".into()),
        };
        assert_eq!(error.to_string(), "forbidden: not a member");
    }
}
