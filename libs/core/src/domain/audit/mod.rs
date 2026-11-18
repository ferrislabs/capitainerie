pub mod ports;
pub mod service;

use capitainerie_common::{AgentId, CardId, CoreError};
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::agent_task::AGENT_ACTOR_PREFIX;

pub const DEFAULT_PAGE: u32 = 50;
pub const MAX_PAGE: u32 = 200;
const TITLE_IN_SUMMARY_CHARS: usize = 80;
const REASON_IN_SUMMARY_CHARS: usize = 160;

/// An outbox event as the audit log reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditEvent {
    pub id: Uuid,
    pub topic: String,
    pub kind: String,
    pub occurred_at: DateTime<Utc>,
    pub payload: Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AuditFilter {
    pub topic: Option<String>,
    pub kind: Option<String>,
    /// FerrisKey subject of a member: the events they caused, themselves or
    /// through an agent acting for them.
    pub actor: Option<String>,
    pub agent_id: Option<AgentId>,
    pub card_id: Option<CardId>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    /// Keyset cursor: only events older than this one.
    pub before: Option<Uuid>,
    pub limit: u32,
}

impl AuditFilter {
    pub fn page(limit: Option<u32>) -> Result<u32, CoreError> {
        match limit {
            None => Ok(DEFAULT_PAGE),
            Some(limit) if (1..=MAX_PAGE).contains(&limit) => Ok(limit),
            Some(_) => Err(CoreError::Invalid(format!(
                "the limit must be between 1 and {MAX_PAGE}"
            ))),
        }
    }

    pub fn check_window(&self) -> Result<(), CoreError> {
        match (self.from, self.to) {
            (Some(from), Some(to)) if from >= to => {
                Err(CoreError::Invalid("`from` must be before `to`".into()))
            }
            _ => Ok(()),
        }
    }
}

/// Who caused an event, before names are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorRef {
    Member(String),
    /// An agent known by its client id (`agent:<slug>` for crew agents).
    AgentActor(String),
    AgentId(Uuid),
    /// The agent of the agent task a run event belongs to.
    RunAgent(Uuid),
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    Member,
    Agent,
    System,
}

impl ActorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Agent => "agent",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub kind: ActorKind,
    /// A member's subject, an agent's id, or its client id when it is not a crew agent.
    pub id: Option<String>,
    pub name: String,
}

impl Actor {
    pub fn system() -> Self {
        Self {
            kind: ActorKind::System,
            id: None,
            name: "Système".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    Warning,
    Info,
    Success,
    Created,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Success => "success",
            Self::Created => "created",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditEntry {
    pub event: AuditEvent,
    pub actor: Actor,
    pub summary: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditPage {
    pub items: Vec<AuditEntry>,
    pub next_cursor: Option<Uuid>,
}

pub(crate) fn text<'a>(payload: &'a Value, field: &str) -> Option<&'a str> {
    payload.get(field).and_then(Value::as_str)
}

pub(crate) fn uuid_in(payload: &Value, field: &str) -> Option<Uuid> {
    text(payload, field).and_then(|value| Uuid::parse_str(value).ok())
}

pub fn actor_of(kind: &str, payload: &Value) -> ActorRef {
    if let Some(author) = payload.get("author") {
        if let Some(actor) = text(author, "actor") {
            return ActorRef::AgentActor(actor.to_owned());
        }
        if let Some(subject) = text(author, "subject") {
            return ActorRef::Member(subject.to_owned());
        }
    }
    match kind {
        "run.queued" => text(payload, "initiated_by")
            .map_or(ActorRef::System, |subject| ActorRef::Member(subject.into())),
        "approval.decided" => text(payload, "decided_by")
            .map_or(ActorRef::System, |subject| ActorRef::Member(subject.into())),
        "approval.requested" => {
            uuid_in(payload, "requested_by_agent_id").map_or(ActorRef::System, ActorRef::AgentId)
        }
        _ if kind.starts_with("run.") => {
            uuid_in(payload, "run_id").map_or(ActorRef::System, ActorRef::RunAgent)
        }
        _ => ActorRef::System,
    }
}

/// The slug of a crew agent actor (`agent:<slug>`).
pub fn crew_slug(actor: &str) -> Option<&str> {
    actor.strip_prefix(AGENT_ACTOR_PREFIX)
}

pub fn severity(kind: &str, payload: &Value) -> Severity {
    let status = text(payload, "status");
    match (kind, status) {
        ("run.finished", Some("failed"))
        | ("source.event_processed", Some("failed"))
        | ("routine.run_skipped", Some("failed")) => Severity::Critical,
        ("run.step_finished", Some("failed")) => Severity::Warning,
        ("routine.run_skipped", _) | ("approval.requested", _) => Severity::Warning,
        ("approval.decided", Some("rejected")) => Severity::Warning,
        ("run.finished", Some("succeeded")) | ("approval.decided", Some("approved")) => {
            Severity::Success
        }
        ("card.proposed", _) | ("routine.run_created", _) | ("run.queued", _) => Severity::Created,
        _ => Severity::Info,
    }
}

fn shortened(value: &str, max: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut cut: String = value.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

fn column_label(column: &str) -> &str {
    match column {
        "proposed" => "Proposées",
        "todo" => "À faire",
        "in_progress" => "En cours",
        "done" => "Terminées",
        "rejected" => "Rejetées",
        other => other,
    }
}

/// What the actor did, in French, as the console prints it after their name.
/// `card_title` names the card the event concerns, when it is known.
pub fn summary(kind: &str, payload: &Value, card_title: Option<&str>) -> String {
    let title = text(payload, "title")
        .or(card_title)
        .map(|title| format!(" « {} »", shortened(title, TITLE_IN_SUMMARY_CHARS)))
        .unwrap_or_default();
    let on_card = if title.is_empty() {
        String::new()
    } else {
        format!(" sur{title}")
    };
    let status = text(payload, "status");
    let reason = |field: &str| {
        text(payload, field)
            .filter(|reason| !reason.trim().is_empty())
            .map(|reason| format!(" : {}", shortened(reason, REASON_IN_SUMMARY_CHARS)))
            .unwrap_or_default()
    };
    match kind {
        "run.queued" => format!("a lancé une exécution{on_card}"),
        "run.started" if payload.get("resumed") == Some(&Value::Bool(true)) => {
            "a repris une exécution".into()
        }
        "run.started" => "a démarré une exécution".into(),
        "run.step_finished" => {
            let step = text(payload, "kind").unwrap_or("?");
            match status {
                Some("failed") => format!("a échoué à l'étape {step}"),
                _ => format!("a terminé l'étape {step}"),
            }
        }
        "run.finished" => match status {
            Some("succeeded") => "a terminé une exécution".into(),
            Some("failed") => format!("a échoué une exécution{}", reason("failure")),
            Some("cancelled") => "a vu une exécution annulée".into(),
            _ => "a clos une exécution".into(),
        },
        "card.proposed" => format!("a proposé la carte{title}"),
        "card.updated" if text(payload, "assigned_to").is_some() => {
            format!("a assigné la carte{title}")
        }
        "card.updated" => format!("a modifié la carte{title}"),
        "card.moved" => format!(
            "a déplacé la carte{title} vers {}",
            column_label(text(payload, "to").unwrap_or("?"))
        ),
        "card.commented" => format!("a commenté la carte{title}"),
        "approval.requested" => format!("a demandé une validation{on_card}"),
        "approval.decided" => match status {
            Some("approved") => format!("a approuvé une demande{on_card}"),
            Some("rejected") => format!("a refusé une demande{on_card}"),
            _ => format!("a tranché une demande{on_card}"),
        },
        "source.event_received" => "a reçu un événement de source".into(),
        "source.event_processed" => match status {
            Some("card_created") => format!("a créé une carte depuis une source{on_card}"),
            Some("ignored") => "a ignoré un événement de source".into(),
            Some("failed") => "n'a pas pu traiter un événement de source".into(),
            _ => "a traité un événement de source".into(),
        },
        "routine.run_created" => format!("a lancé un passage de routine{on_card}"),
        "routine.run_skipped" => match status {
            Some("failed") => format!("a échoué un passage de routine{}", reason("reason")),
            _ => format!("a sauté un passage de routine{}", reason("reason")),
        },
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_actor_comes_from_the_author_the_initiator_or_the_decider() {
        let card = json!({"author": {"subject": "user-1", "actor": null}});
        assert_eq!(
            actor_of("card.moved", &card),
            ActorRef::Member("user-1".into())
        );
        let by_agent = json!({"author": {"subject": "user-1", "actor": "agent:iris"}});
        assert_eq!(
            actor_of("card.commented", &by_agent),
            ActorRef::AgentActor("agent:iris".into())
        );
        assert_eq!(
            actor_of("run.queued", &json!({"initiated_by": "user-2"})),
            ActorRef::Member("user-2".into())
        );
        assert_eq!(
            actor_of("approval.decided", &json!({"decided_by": "user-3"})),
            ActorRef::Member("user-3".into())
        );
        let agent = Uuid::now_v7();
        assert_eq!(
            actor_of(
                "approval.requested",
                &json!({"requested_by_agent_id": agent})
            ),
            ActorRef::AgentId(agent)
        );
        let run = Uuid::now_v7();
        assert_eq!(
            actor_of("run.finished", &json!({"run_id": run})),
            ActorRef::RunAgent(run)
        );
        assert_eq!(
            actor_of("source.event_received", &json!({})),
            ActorRef::System
        );
        assert_eq!(crew_slug("agent:iris"), Some("iris"));
        assert_eq!(crew_slug("capitainerie-captain"), None);
    }

    #[test]
    fn failures_are_critical_and_creations_are_marked() {
        assert_eq!(
            severity("run.finished", &json!({"status": "failed"})),
            Severity::Critical
        );
        assert_eq!(
            severity("run.finished", &json!({"status": "succeeded"})),
            Severity::Success
        );
        assert_eq!(
            severity("routine.run_skipped", &json!({"status": "skipped"})),
            Severity::Warning
        );
        assert_eq!(severity("card.proposed", &json!({})), Severity::Created);
        assert_eq!(severity("card.commented", &json!({})), Severity::Info);
    }

    #[test]
    fn a_summary_names_the_card_and_falls_back_to_the_kind() {
        assert_eq!(
            summary(
                "card.proposed",
                &json!({"title": "Relancer le client"}),
                None
            ),
            "a proposé la carte « Relancer le client »"
        );
        assert_eq!(
            summary("card.moved", &json!({"to": "done"}), Some("Facture")),
            "a déplacé la carte « Facture » vers Terminées"
        );
        assert_eq!(
            summary("card.updated", &json!({"assigned_to": "user-2"}), Some("X")),
            "a assigné la carte « X »"
        );
        assert_eq!(
            summary(
                "run.finished",
                &json!({"status": "failed", "failure": "délai dépassé"}),
                None
            ),
            "a échoué une exécution : délai dépassé"
        );
        assert_eq!(
            summary("run.queued", &json!({}), Some("Revue")),
            "a lancé une exécution sur « Revue »"
        );
        assert_eq!(
            summary("thing.happened", &json!({}), None),
            "thing.happened"
        );
        let long = "a".repeat(200);
        assert!(
            summary("card.commented", &json!({}), Some(&long))
                .chars()
                .count()
                < 120
        );
    }

    #[test]
    fn the_page_size_and_the_window_are_bounded() {
        assert_eq!(AuditFilter::page(None).unwrap(), DEFAULT_PAGE);
        assert_eq!(AuditFilter::page(Some(MAX_PAGE)).unwrap(), MAX_PAGE);
        assert!(AuditFilter::page(Some(0)).is_err());
        assert!(AuditFilter::page(Some(MAX_PAGE + 1)).is_err());
        let now = Utc::now();
        let backwards = AuditFilter {
            from: Some(now),
            to: Some(now),
            ..AuditFilter::default()
        };
        assert!(backwards.check_window().is_err());
    }
}
