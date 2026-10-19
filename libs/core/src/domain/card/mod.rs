pub mod detail;
pub mod ports;
pub mod service;

use capitainerie_common::{CardCommentId, CardId, CoreError, OrganizationId, RunId, TranscriptId};
use capitainerie_events::card::CardAuthor;
use chrono::{DateTime, Utc};

use crate::domain::transcript::TranscriptBody;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    Proposed,
    Todo,
    InProgress,
    Done,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub id: CardId,
    pub organization_id: OrganizationId,
    pub title: CardTitle,
    pub description: String,
    pub column: Column,
    pub provenance: Option<Provenance>,
    pub created_by_run: Option<RunId>,
    pub created_by: Author,
    /// Optimistic lock: every change increments it, and a change made against
    /// a stale version is refused.
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The exact passage of a transcript a card comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub transcript_id: TranscriptId,
    /// Character offsets `[start, end)` into the transcript body.
    pub start: u32,
    pub end: u32,
    pub quote: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub subject: String,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardComment {
    pub id: CardCommentId,
    pub card_id: CardId,
    pub organization_id: OrganizationId,
    pub author: Author,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardTitle(String);

impl CardTitle {
    pub const MAX_CHARS: usize = 200;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CardTitle {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Invalid("the card title is empty".into()));
        }
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(CoreError::Invalid(format!(
                "the card title exceeds {} characters",
                Self::MAX_CHARS
            )));
        }
        Ok(Self(trimmed.to_owned()))
    }
}

pub const DESCRIPTION_MAX_CHARS: usize = 10_000;
pub const COMMENT_MAX_CHARS: usize = 10_000;

pub fn bounded_text(
    value: String,
    max: usize,
    what: &str,
    allow_empty: bool,
) -> Result<String, CoreError> {
    if !allow_empty && value.trim().is_empty() {
        return Err(CoreError::Invalid(format!("the {what} is empty")));
    }
    if value.chars().count() > max {
        return Err(CoreError::Invalid(format!(
            "the {what} exceeds {max} characters"
        )));
    }
    Ok(value)
}

impl Provenance {
    /// Checks the quote is exactly the transcript text at `[start, end)`: an
    /// agent cannot attribute to the meeting words nobody said.
    pub fn verified(
        transcript_id: TranscriptId,
        start: u32,
        end: u32,
        quote: String,
        body: &TranscriptBody,
    ) -> Result<Self, CoreError> {
        if end <= start {
            return Err(CoreError::Invalid("the provenance range is empty".into()));
        }
        let passage: String = body
            .as_str()
            .chars()
            .skip(start as usize)
            .take((end - start) as usize)
            .collect();
        if passage.chars().count() != (end - start) as usize {
            return Err(CoreError::Invalid(
                "the provenance range goes past the end of the transcript".into(),
            ));
        }
        if passage != quote {
            return Err(CoreError::Invalid(format!(
                "the quote does not match the transcript at [{start}, {end}): the text there is {passage:?}"
            )));
        }
        Ok(Self {
            transcript_id,
            start,
            end,
            quote,
        })
    }
}

impl Card {
    pub fn touch(&mut self, now: DateTime<Utc>) {
        self.version += 1;
        self.updated_at = now;
    }

    pub fn ensure_version(&self, expected: i32) -> Result<(), CoreError> {
        if self.version != expected {
            return Err(CoreError::Conflict(format!(
                "card {} is at version {}, not {expected}: read it again",
                self.id, self.version
            )));
        }
        Ok(())
    }
}

impl Column {
    pub const ALL: [Column; 5] = [
        Column::Proposed,
        Column::Todo,
        Column::InProgress,
        Column::Done,
        Column::Rejected,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Rejected => "rejected",
        }
    }
}

impl TryFrom<&str> for Column {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|column| column.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown column {value}")))
    }
}

impl From<&Author> for CardAuthor {
    fn from(author: &Author) -> Self {
        Self {
            subject: author.subject.clone(),
            actor: author.actor.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(text: &str) -> TranscriptBody {
        TranscriptBody::try_from(text.to_string()).unwrap()
    }

    #[test]
    fn provenance_accepts_the_exact_passage_in_characters() {
        let transcript = body("Élodie : je relance Acme demain.");
        let provenance = Provenance::verified(
            TranscriptId::new(),
            9,
            31,
            "je relance Acme demain".into(),
            &transcript,
        )
        .unwrap();
        assert_eq!(provenance.quote, "je relance Acme demain");
    }

    #[test]
    fn provenance_rejects_an_invented_quote() {
        let transcript = body("Élodie : je relance Acme demain.");
        let error = Provenance::verified(
            TranscriptId::new(),
            9,
            31,
            "je relance Globex demain".into(),
            &transcript,
        )
        .unwrap_err();
        assert!(
            matches!(error, CoreError::Invalid(message) if message.contains("je relance Acme demain"))
        );
    }

    #[test]
    fn provenance_rejects_ranges_outside_the_transcript() {
        let transcript = body("court");
        assert!(
            Provenance::verified(TranscriptId::new(), 2, 2, String::new(), &transcript).is_err()
        );
        assert!(
            Provenance::verified(TranscriptId::new(), 3, 10, "rt".into(), &transcript).is_err()
        );
    }

    #[test]
    fn columns_round_trip_and_reject_unknown_names() {
        for column in Column::ALL {
            assert_eq!(Column::try_from(column.as_str()).unwrap(), column);
        }
        assert!(Column::try_from("backlog").is_err());
    }

    #[test]
    fn card_titles_are_trimmed_and_bounded() {
        assert_eq!(
            CardTitle::try_from("  Relancer Acme ".to_string())
                .unwrap()
                .as_str(),
            "Relancer Acme"
        );
        assert!(CardTitle::try_from(" ".to_string()).is_err());
        assert!(CardTitle::try_from("x".repeat(CardTitle::MAX_CHARS + 1)).is_err());
    }
}
