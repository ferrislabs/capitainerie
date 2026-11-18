pub mod ports;
pub mod projection;
pub mod service;

use capitainerie_common::{CardId, CoreError, InboxItemId, OrganizationId, RunId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub const DEFAULT_PAGE: u32 = 50;
pub const MAX_PAGE: u32 = 200;
pub const MAX_READ_IDS: usize = 500;
pub const TITLE_MAX_CHARS: usize = 300;
pub const BODY_MAX_CHARS: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxKind {
    Approval,
    Run,
    Card,
    Source,
    Routine,
}

impl InboxKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approval => "approval",
            Self::Run => "run",
            Self::Card => "card",
            Self::Source => "source",
            Self::Routine => "routine",
        }
    }
}

impl TryFrom<&str> for InboxKind {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "approval" => Ok(Self::Approval),
            "run" => Ok(Self::Run),
            "card" => Ok(Self::Card),
            "source" => Ok(Self::Source),
            "routine" => Ok(Self::Routine),
            other => Err(CoreError::Invalid(format!("unknown inbox kind {other}"))),
        }
    }
}

/// What one event tells one member, before it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub member_subject: String,
    pub kind: InboxKind,
    pub title: String,
    pub body: String,
    /// A console path.
    pub link: Option<String>,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxItem {
    pub id: InboxItemId,
    pub organization_id: OrganizationId,
    pub member_subject: String,
    /// The outbox event it was projected from.
    pub event_id: Uuid,
    pub kind: InboxKind,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

fn clipped(value: &str, max: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut cut: String = value.chars().take(max - 1).collect();
    cut.push('…');
    cut
}

impl InboxItem {
    pub fn project(
        organization_id: OrganizationId,
        event_id: Uuid,
        notification: Notification,
        now: DateTime<Utc>,
    ) -> Self {
        let title = clipped(&notification.title, TITLE_MAX_CHARS);
        Self {
            id: InboxItemId::new(),
            organization_id,
            member_subject: notification.member_subject,
            event_id,
            kind: notification.kind,
            title: if title.is_empty() {
                "Notification".into()
            } else {
                title
            },
            body: clipped(&notification.body, BODY_MAX_CHARS),
            link: notification.link,
            card_id: notification.card_id,
            run_id: notification.run_id,
            created_at: now,
            read_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxPage {
    pub items: Vec<InboxItem>,
    pub next_cursor: Option<InboxItemId>,
    pub total: i64,
    pub unread: i64,
}

/// Which items to mark read: some, or every one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadSelection {
    Some(Vec<InboxItemId>),
    All,
}

impl ReadSelection {
    pub fn new(ids: Option<Vec<InboxItemId>>, all: bool) -> Result<Self, CoreError> {
        match (ids, all) {
            (Some(ids), false) if ids.is_empty() => {
                Err(CoreError::Invalid("give `ids` or `all`".into()))
            }
            (Some(ids), false) if ids.len() > MAX_READ_IDS => Err(CoreError::Invalid(format!(
                "at most {MAX_READ_IDS} ids at once"
            ))),
            (Some(ids), false) => Ok(Self::Some(ids)),
            (None, true) => Ok(Self::All),
            (Some(_), true) => Err(CoreError::Invalid("give `ids` or `all`, not both".into())),
            (None, false) => Err(CoreError::Invalid("give `ids` or `all`".into())),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_projected_item_is_unread_and_bounded() {
        let item = InboxItem::project(
            OrganizationId::new(),
            Uuid::now_v7(),
            Notification {
                member_subject: "user-1".into(),
                kind: InboxKind::Run,
                title: "t".repeat(TITLE_MAX_CHARS + 10),
                body: "  corps  ".into(),
                link: None,
                card_id: None,
                run_id: None,
            },
            Utc::now(),
        );
        assert_eq!(item.title.chars().count(), TITLE_MAX_CHARS);
        assert!(item.title.ends_with('…'));
        assert_eq!(item.body, "corps");
        assert_eq!(item.read_at, None);
    }

    #[test]
    fn a_read_selection_is_ids_or_all() {
        assert_eq!(ReadSelection::new(None, true).unwrap(), ReadSelection::All);
        let id = InboxItemId::new();
        assert_eq!(
            ReadSelection::new(Some(vec![id]), false).unwrap(),
            ReadSelection::Some(vec![id])
        );
        assert!(ReadSelection::new(None, false).is_err());
        assert!(ReadSelection::new(Some(vec![]), false).is_err());
        assert!(ReadSelection::new(Some(vec![id]), true).is_err());
        assert!(ReadSelection::new(Some(vec![id; MAX_READ_IDS + 1]), false).is_err());
    }

    #[test]
    fn kinds_round_trip() {
        for kind in [
            InboxKind::Approval,
            InboxKind::Run,
            InboxKind::Card,
            InboxKind::Source,
            InboxKind::Routine,
        ] {
            assert_eq!(InboxKind::try_from(kind.as_str()).unwrap(), kind);
        }
        assert!(InboxKind::try_from("mr").is_err());
    }
}
