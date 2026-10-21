use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("resource not found")]
    NotFound,

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("forbidden{}", .reason.as_ref().map(|r| format!(": {r}")).unwrap_or_default())]
    Forbidden { reason: Option<String> },

    #[error("database error: {0}")]
    Database(String),

    #[error("internal error: {0}")]
    Internal(String),
}

macro_rules! id {
    ($name:ident) => {
        id!($name, Uuid::now_v7);
    };
    ($name:ident, $generate:path) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self($generate())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id!(OrganizationId);
id!(TranscriptId);
id!(RunId);
id!(CardId);
id!(CardCommentId);
id!(StepId);
id!(ClientId, Uuid::new_v4);
id!(ProjectId, Uuid::new_v4);
id!(ConversationId);
id!(ConversationMessageId);
id!(IntegrationInstanceId);
id!(IntegrationActionId);

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
