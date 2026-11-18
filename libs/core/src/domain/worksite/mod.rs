pub mod ports;

use capitainerie_common::{CardId, OrganizationId, RunId};
use chrono::{DateTime, Utc};

use crate::domain::execution::RepositoryRef;

/// What a card of a worksite was made from, when it was made from an issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorksiteIssue {
    pub card_id: CardId,
    pub organization_id: OrganizationId,
    pub repository: RepositoryRef,
    pub number: u64,
    pub url: String,
    pub created_at: DateTime<Utc>,
}

/// One line of a project's worksites: the card, the issue it came from and
/// the latest execution of the card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorksiteSummary {
    pub card_id: CardId,
    pub title: String,
    pub column: String,
    pub issue_number: Option<u64>,
    pub issue_url: Option<String>,
    pub repository: Option<RepositoryRef>,
    pub run_id: Option<RunId>,
    pub run_status: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A worksite to open: a free prompt, or an open issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorksiteDraft {
    pub title: String,
    pub description: String,
    pub issue: Option<DraftIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftIssue {
    pub number: u64,
    pub url: String,
}

/// The most worksites one call may open.
pub const MAX_WORKSITES_PER_LAUNCH: usize = 20;

impl WorksiteDraft {
    /// A prompt becomes a card whose title is its first line.
    pub fn from_prompt(prompt: &str) -> Self {
        let prompt = prompt.trim();
        let first_line = prompt.lines().next().unwrap_or_default().trim();
        let title: String = first_line
            .chars()
            .take(crate::domain::card::CardTitle::MAX_CHARS)
            .collect();
        Self {
            title,
            description: prompt.to_owned(),
            issue: None,
        }
    }

    pub fn from_issue(issue: &crate::domain::execution::RepositoryIssue) -> Self {
        let mut description = issue.body.trim().to_owned();
        if !description.is_empty() {
            description.push_str("\n\n");
        }
        description.push_str(&format!("Issue #{}: {}", issue.number, issue.html_url));
        Self {
            title: issue
                .title
                .chars()
                .take(crate::domain::card::CardTitle::MAX_CHARS)
                .collect(),
            description,
            issue: Some(DraftIssue {
                number: issue.number,
                url: issue.html_url.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::execution::RepositoryIssue;

    #[test]
    fn a_prompt_is_titled_by_its_first_line() {
        let draft =
            WorksiteDraft::from_prompt("  Add a health check\n\nOn /healthz, 200 when ready.");
        assert_eq!(draft.title, "Add a health check");
        assert!(draft.description.ends_with("200 when ready."));
        assert!(draft.issue.is_none());
    }

    #[test]
    fn a_long_first_line_is_cut_to_the_title_limit() {
        let draft = WorksiteDraft::from_prompt(&"a".repeat(500));
        assert_eq!(draft.title.chars().count(), 200);
    }

    #[test]
    fn an_issue_keeps_its_link_in_the_description() {
        let draft = WorksiteDraft::from_issue(&RepositoryIssue {
            number: 12,
            title: "Crash on empty input".into(),
            body: "Steps…".into(),
            html_url: "https://github.com/acme/api/issues/12".into(),
            labels: vec![],
        });
        assert_eq!(draft.title, "Crash on empty input");
        assert!(draft.description.starts_with("Steps…"));
        assert!(draft.description.ends_with("issues/12"));
        assert_eq!(draft.issue.unwrap().number, 12);
    }

    #[test]
    fn an_issue_without_body_is_just_the_link() {
        let draft = WorksiteDraft::from_issue(&RepositoryIssue {
            number: 3,
            title: "x".into(),
            body: String::new(),
            html_url: "https://github.com/acme/api/issues/3".into(),
            labels: vec![],
        });
        assert_eq!(
            draft.description,
            "Issue #3: https://github.com/acme/api/issues/3"
        );
    }
}
