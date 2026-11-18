pub mod ports;
pub mod service;

use capitainerie_common::{AgentId, ArtifactId, CardId, CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};

pub const CONTENT_MAX_BYTES: usize = 1024 * 1024;
pub const TITLE_MAX_CHARS: usize = 300;
pub const DEFAULT_PAGE: u32 = 50;
pub const MAX_PAGE: u32 = 200;
const TRUNCATION_NOTE: &str = "\n\n_[contenu tronqué]_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Markdown,
    Json,
    Link,
}

impl ArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Link => "link",
        }
    }
}

impl TryFrom<&str> for ArtifactKind {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "markdown" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            "link" => Ok(Self::Link),
            other => Err(CoreError::Invalid(format!("unknown artifact kind {other}"))),
        }
    }
}

/// A deliverable an agent produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub id: ArtifactId,
    pub organization_id: OrganizationId,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    pub agent_id: Option<AgentId>,
    pub kind: ArtifactKind,
    pub title: String,
    /// The text, for every kind but `link`.
    pub content: Option<String>,
    /// The target, for a `link`.
    pub url: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An artifact with the names the console shows next to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactView {
    pub artifact: Artifact,
    pub agent_name: Option<String>,
    pub card_title: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactFilter {
    pub card_id: Option<CardId>,
    pub agent_id: Option<AgentId>,
    pub kind: Option<ArtifactKind>,
    pub before: Option<ArtifactId>,
    pub limit: u32,
}

pub fn page(limit: Option<u32>) -> Result<u32, CoreError> {
    match limit {
        None => Ok(DEFAULT_PAGE),
        Some(limit) if (1..=MAX_PAGE).contains(&limit) => Ok(limit),
        Some(_) => Err(CoreError::Invalid(format!(
            "the limit must be between 1 and {MAX_PAGE}"
        ))),
    }
}

fn bounded_title(title: &str) -> String {
    let title = title.trim();
    if title.chars().count() <= TITLE_MAX_CHARS {
        return title.to_owned();
    }
    let mut cut: String = title.chars().take(TITLE_MAX_CHARS - 1).collect();
    cut.push('…');
    cut
}

/// Keeps `content` under the size limit, cut on a character boundary.
fn bounded_content(content: &str) -> String {
    if content.len() <= CONTENT_MAX_BYTES {
        return content.to_owned();
    }
    let mut end = CONTENT_MAX_BYTES - TRUNCATION_NOTE.len();
    while !content.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{TRUNCATION_NOTE}", &content[..end])
}

/// Where a run's report is filed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportOrigin {
    pub organization_id: OrganizationId,
    pub card_id: CardId,
    pub run_id: RunId,
    pub agent_id: AgentId,
}

impl Artifact {
    /// The final answer of an agent task run, as a markdown document. `None`
    /// when the answer is blank.
    pub fn report(
        origin: ReportOrigin,
        card_title: &str,
        text: &str,
        created_by: String,
        now: DateTime<Utc>,
    ) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let title = bounded_title(card_title);
        Some(Self {
            id: ArtifactId::new(),
            organization_id: origin.organization_id,
            card_id: Some(origin.card_id),
            run_id: Some(origin.run_id),
            agent_id: Some(origin.agent_id),
            kind: ArtifactKind::Markdown,
            title: if title.is_empty() {
                "Rapport".into()
            } else {
                title
            },
            content: Some(bounded_content(text)),
            url: None,
            created_by,
            created_at: now,
            updated_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> ReportOrigin {
        ReportOrigin {
            organization_id: OrganizationId::new(),
            card_id: CardId::new(),
            run_id: RunId::new(),
            agent_id: AgentId::new(),
        }
    }

    #[test]
    fn a_report_is_a_markdown_document_named_after_the_card() {
        let origin = origin();
        let report = Artifact::report(
            origin,
            "Plan de la semaine",
            " # Plan\n",
            "user-1".into(),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(report.kind, ArtifactKind::Markdown);
        assert_eq!(report.title, "Plan de la semaine");
        assert_eq!(report.content.as_deref(), Some("# Plan"));
        assert_eq!(report.run_id, Some(origin.run_id));
        assert_eq!(report.agent_id, Some(origin.agent_id));
        assert_eq!(report.url, None);
        assert!(Artifact::report(origin, "x", "  ", "user-1".into(), Utc::now()).is_none());
    }

    #[test]
    fn a_huge_report_is_cut_on_a_character_boundary() {
        let text = "é".repeat(CONTENT_MAX_BYTES);
        let report = Artifact::report(origin(), "x", &text, "user-1".into(), Utc::now()).unwrap();
        let content = report.content.unwrap();
        assert!(content.len() <= CONTENT_MAX_BYTES);
        assert!(content.ends_with(TRUNCATION_NOTE));
        let long_title = "t".repeat(TITLE_MAX_CHARS * 2);
        let report = Artifact::report(origin(), &long_title, "ok", "u".into(), Utc::now()).unwrap();
        assert_eq!(report.title.chars().count(), TITLE_MAX_CHARS);
    }

    #[test]
    fn kinds_round_trip_and_pages_are_bounded() {
        for kind in [
            ArtifactKind::Markdown,
            ArtifactKind::Json,
            ArtifactKind::Link,
        ] {
            assert_eq!(ArtifactKind::try_from(kind.as_str()).unwrap(), kind);
        }
        assert!(ArtifactKind::try_from("pdf").is_err());
        assert_eq!(page(None).unwrap(), DEFAULT_PAGE);
        assert!(page(Some(0)).is_err());
        assert!(page(Some(MAX_PAGE + 1)).is_err());
    }
}
