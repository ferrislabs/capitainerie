pub mod ports;

use std::borrow::Cow;

use capitainerie_common::{CoreError, OrganizationId, TranscriptId};
use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::{
    client::{catalog::Catalog, links::Links},
    execution::RepositoryRef,
    label::clean_label,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    pub id: TranscriptId,
    pub organization_id: OrganizationId,
    /// FerrisKey subject of the human who submitted it.
    pub submitted_by: String,
    pub title: TranscriptTitle,
    pub body: TranscriptBody,
    pub metadata: MeetingMetadata,
    pub links: Links,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeetingMetadata {
    pub date: Option<NaiveDate>,
    pub participants: Vec<String>,
    pub repository: Option<RepositoryRef>,
}

impl MeetingMetadata {
    pub const MAX_PARTICIPANTS: usize = 50;
    pub const MAX_PARTICIPANT_CHARS: usize = 100;

    pub fn new(
        date: Option<NaiveDate>,
        participants: Option<Vec<String>>,
        repository: Option<String>,
    ) -> Result<Self, CoreError> {
        let participants = participants.unwrap_or_default();
        if participants.len() > Self::MAX_PARTICIPANTS {
            return Err(CoreError::Invalid(format!(
                "there are more than {} participants",
                Self::MAX_PARTICIPANTS
            )));
        }
        let mut participants: Vec<String> = participants
            .iter()
            .map(|name| clean_label(name, Self::MAX_PARTICIPANT_CHARS, "a participant name"))
            .collect::<Result<_, _>>()?;
        let mut seen = std::collections::HashSet::new();
        participants.retain(|name| seen.insert(name.to_lowercase()));
        let repository = repository
            .map(|value| value.trim().parse::<RepositoryRef>())
            .transpose()?;
        Ok(Self {
            date,
            participants,
            repository,
        })
    }

    pub fn header(&self) -> Option<String> {
        let mut lines = Vec::new();
        if let Some(date) = self.date {
            lines.push(format!("Date de la réunion : {date}"));
        }
        if !self.participants.is_empty() {
            lines.push(format!("Participants : {}", self.participants.join(", ")));
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    pub fn prompt_for<'a>(&self, body: &'a str) -> Cow<'a, str> {
        self.prompt_with(body, None)
    }

    pub fn prompt_with<'a>(&self, body: &'a str, catalog: Option<&Catalog>) -> Cow<'a, str> {
        let parts: Vec<String> = self
            .header()
            .into_iter()
            .chain(catalog.and_then(Catalog::block))
            .collect();
        if parts.is_empty() {
            Cow::Borrowed(body)
        } else {
            Cow::Owned(format!("{}\n\n{body}", parts.join("\n\n")))
        }
    }
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

    fn names(values: &[&str]) -> Option<Vec<String>> {
        Some(values.iter().map(|value| (*value).to_owned()).collect())
    }

    #[test]
    fn no_metadata_is_empty() {
        let metadata = MeetingMetadata::new(None, None, None).unwrap();
        assert_eq!(metadata.header(), None);
        assert_eq!(metadata, MeetingMetadata::default());
    }

    #[test]
    fn participants_are_trimmed() {
        let metadata = MeetingMetadata::new(None, names(&["  Alice ", "Bob"]), None).unwrap();
        assert_eq!(metadata.participants, ["Alice", "Bob"]);
        assert_eq!(metadata.header().unwrap(), "Participants : Alice, Bob");
    }

    #[test]
    fn a_blank_participant_is_rejected() {
        assert!(MeetingMetadata::new(None, names(&["Alice", "  "]), None).is_err());
    }

    #[test]
    fn participant_limits_count_characters() {
        let longest = "é".repeat(MeetingMetadata::MAX_PARTICIPANT_CHARS);
        assert!(MeetingMetadata::new(None, Some(vec![longest.clone()]), None).is_ok());
        assert!(MeetingMetadata::new(None, Some(vec![format!("{longest}é")]), None).is_err());
    }

    #[test]
    fn at_most_fifty_participants() {
        let many = |count: usize| Some((0..count).map(|n| format!("P{n}")).collect::<Vec<_>>());
        assert!(MeetingMetadata::new(None, many(MeetingMetadata::MAX_PARTICIPANTS), None).is_ok());
        assert!(
            MeetingMetadata::new(None, many(MeetingMetadata::MAX_PARTICIPANTS + 1), None).is_err()
        );
    }

    #[test]
    fn the_repository_is_validated() {
        let metadata = MeetingMetadata::new(None, None, Some(" acme/web ".into())).unwrap();
        assert_eq!(metadata.repository.unwrap().to_string(), "acme/web");
        assert!(MeetingMetadata::new(None, None, Some("acme".into())).is_err());
        assert!(MeetingMetadata::new(None, None, Some("../etc".into())).is_err());
    }

    #[test]
    fn the_repository_alone_is_not_sent_to_the_model() {
        let metadata = MeetingMetadata::new(None, None, Some("acme/web".into())).unwrap();
        assert_eq!(metadata.header(), None);
        assert_eq!(metadata.prompt_for("Alice : ok"), "Alice : ok");
    }

    #[test]
    fn the_header_is_in_french_and_precedes_the_transcript() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 19);
        let metadata = MeetingMetadata::new(date, names(&["Alice", "Bob"]), None).unwrap();
        assert_eq!(
            metadata.header().unwrap(),
            "Date de la réunion : 2026-10-19\nParticipants : Alice, Bob"
        );
        assert_eq!(
            metadata.prompt_for("Alice : ok"),
            "Date de la réunion : 2026-10-19\nParticipants : Alice, Bob\n\nAlice : ok"
        );
    }

    #[test]
    fn the_header_skips_what_is_missing() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 19);
        let metadata = MeetingMetadata::new(date, None, None).unwrap();
        assert_eq!(
            metadata.header().unwrap(),
            "Date de la réunion : 2026-10-19"
        );
    }

    #[test]
    fn participant_names_cannot_carry_line_breaks_or_commas() {
        for name in [
            "A\nB",
            "A\rB",
            "A\u{2028}B",
            "A\u{2029}B",
            "A\u{0}B",
            "A\tB",
            "Dupont, Jean",
        ] {
            assert!(
                MeetingMetadata::new(None, Some(vec![name.to_owned()]), None).is_err(),
                "{name:?}"
            );
        }
    }

    #[test]
    fn participants_are_deduplicated_case_insensitively_keeping_the_first() {
        let metadata = MeetingMetadata::new(
            None,
            names(&["Alice", "bob", "ALICE", " Bob ", "Claire"]),
            None,
        )
        .unwrap();
        assert_eq!(metadata.participants, ["Alice", "bob", "Claire"]);
    }
}
