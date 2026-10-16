pub mod ports;

use capitainerie_common::{CoreError, OrganizationId, TranscriptId};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    pub id: TranscriptId,
    pub organization_id: OrganizationId,
    /// FerrisKey subject of the human who submitted it.
    pub submitted_by: String,
    pub title: TranscriptTitle,
    pub body: TranscriptBody,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptTitle(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptBody(String);

impl TranscriptTitle {
    pub const MAX_CHARS: usize = 200;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TranscriptBody {
    pub const MAX_CHARS: usize = 200_000;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TranscriptTitle {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Invalid("the title is empty".into()));
        }
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(CoreError::Invalid(format!(
                "the title exceeds {} characters",
                Self::MAX_CHARS
            )));
        }
        Ok(Self(trimmed.to_owned()))
    }
}

impl TryFrom<String> for TranscriptBody {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(CoreError::Invalid("the transcript is empty".into()));
        }
        if value.chars().count() > Self::MAX_CHARS {
            return Err(CoreError::Invalid(format!(
                "the transcript exceeds {} characters",
                Self::MAX_CHARS
            )));
        }
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_trimmed() {
        let title = TranscriptTitle::try_from("  Comité du lundi \n".to_string()).unwrap();
        assert_eq!(title.as_str(), "Comité du lundi");
    }

    #[test]
    fn blank_title_and_body_are_rejected() {
        assert!(TranscriptTitle::try_from("   ".to_string()).is_err());
        assert!(TranscriptBody::try_from("\n\t ".to_string()).is_err());
    }

    #[test]
    fn limits_count_characters_not_bytes() {
        let accented = "é".repeat(TranscriptTitle::MAX_CHARS);
        assert!(TranscriptTitle::try_from(accented).is_ok());
        assert!(TranscriptTitle::try_from("é".repeat(TranscriptTitle::MAX_CHARS + 1)).is_err());
    }

    #[test]
    fn body_keeps_its_exact_text() {
        let body = TranscriptBody::try_from("  Alice : on livre vendredi.\n".to_string()).unwrap();
        assert_eq!(body.as_str(), "  Alice : on livre vendredi.\n");
    }
}
