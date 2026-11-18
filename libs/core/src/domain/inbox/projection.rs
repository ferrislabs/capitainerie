//! Turns outbox events into the members' notifications. The projection reads
//! the outbox in id order behind a cursor; an event is read once it is
//! `settle` old, so a transaction that commits within that delay after
//! minting its event ids is not skipped. Replaying is harmless: a member gets
//! at most one item per event.

use capitainerie_common::{CardId, CoreError, OrganizationId, RunId};
use capitainerie_events::{
    DomainEvent, EventEnvelope,
    approval::ApprovalRequested,
    card::{CardCommented, CardUpdated},
    inbox::InboxItemCreated,
    routine::RoutineRunSkipped,
    run::RunFinished,
    source::SourceEventProcessed,
};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;

use super::{
    InboxItem, InboxKind, Notification,
    ports::{CardFacts, InboxFeed, OwnedFacts, RunFacts},
};
use crate::domain::{
    agent_task::{AGENT_ACTOR_PREFIX, is_agent_actor},
    event::ports::EventEmitter,
};

pub const INBOX_PROJECTION: &str = "inbox";

fn card_link(card_id: uuid::Uuid) -> String {
    format!("/tasks/{card_id}")
}

pub fn approval_requested(
    event: &ApprovalRequested,
    recipient: String,
    title: String,
) -> Notification {
    Notification {
        member_subject: recipient,
        kind: InboxKind::Approval,
        title,
        body: "Une validation attend votre décision".into(),
        link: Some("/approvals".into()),
        card_id: event.card_id.map(CardId),
        run_id: event.run_id.map(RunId),
    }
}

/// The member who started an agent task hears how it ended.
pub fn run_finished(event: &RunFinished, run: &RunFacts) -> Option<Notification> {
    let agent = run.agent_name.as_deref()?;
    let card = run
        .card_title
        .as_deref()
        .map(|title| format!(" « {title} »"))
        .unwrap_or_default();
    let (title, body) = match event.status.as_str() {
        "succeeded" => (format!("{agent} a terminé{card}"), String::new()),
        "failed" => (
            format!("{agent} a échoué sur{card}"),
            event.failure.clone().unwrap_or_default(),
        ),
        _ => return None,
    };
    Some(Notification {
        member_subject: run.initiated_by.clone(),
        kind: InboxKind::Run,
        title,
        body,
        link: run.card_id.map(card_link),
        card_id: run.card_id.map(CardId),
        run_id: Some(RunId(event.run_id)),
    })
}

/// The new assignee hears of it, unless they assigned the card themselves.
pub fn card_assigned(
    event: &CardUpdated,
    card: &CardFacts,
    agent: Option<&str>,
) -> Option<Notification> {
    let assignee = event.assigned_to.as_deref()?;
    if event.author.actor.is_none() && event.author.subject == assignee {
        return None;
    }
    Some(Notification {
        member_subject: assignee.to_owned(),
        kind: InboxKind::Card,
        title: format!("Carte assignée : « {} »", card.title),
        body: agent
            .map(|agent| format!("par {agent}"))
            .unwrap_or_default(),
        link: Some(card_link(event.card_id)),
        card_id: Some(CardId(event.card_id)),
        run_id: event.run_id.map(RunId),
    })
}

/// The member who created a card hears an agent commented on it, unless the
/// comment ends an agent task they started: the run's notification says it.
pub fn agent_commented(
    event: &CardCommented,
    card: &CardFacts,
    agent: &str,
    run: Option<&RunFacts>,
) -> Option<Notification> {
    event
        .author
        .actor
        .as_deref()
        .filter(|actor| is_agent_actor(actor))?;
    if run.is_some_and(|run| run.agent_name.is_some() && run.initiated_by == card.created_by) {
        return None;
    }
    Some(Notification {
        member_subject: card.created_by.clone(),
        kind: InboxKind::Card,
        title: format!("{agent} a commenté « {} »", card.title),
        body: String::new(),
        link: Some(card_link(event.card_id)),
        card_id: Some(CardId(event.card_id)),
        run_id: event.run_id.map(RunId),
    })
}

pub fn source_failed(event: &SourceEventProcessed, source: &OwnedFacts) -> Option<Notification> {
    (event.status == "failed").then(|| Notification {
        member_subject: source.created_by.clone(),
        kind: InboxKind::Source,
        title: format!("Source « {} » : un événement a échoué", source.name),
        body: String::new(),
        link: Some("/sources".into()),
        card_id: event.card_id.map(CardId),
        run_id: event.run_id.map(RunId),
    })
}

pub fn routine_failed(event: &RoutineRunSkipped, routine: &OwnedFacts) -> Option<Notification> {
    (event.status == "failed").then(|| Notification {
        member_subject: routine.created_by.clone(),
        kind: InboxKind::Routine,
        title: format!("Routine « {} » en échec", routine.name),
        body: event.reason.clone(),
        link: Some("/routines".into()),
        card_id: None,
        run_id: None,
    })
}

pub struct InboxProjection<'a, F, E> {
    feed: F,
    emitter: &'a E,
}

fn open<Ev: DomainEvent + DeserializeOwned>(envelope: &EventEnvelope) -> Option<Ev> {
    match envelope.open::<Ev>()? {
        Ok(event) => Some(event),
        Err(error) => {
            tracing::warn!(event_id = %envelope.id, kind = %envelope.kind, %error, "an outbox event does not decode");
            None
        }
    }
}

impl<'a, F: InboxFeed, E: EventEmitter> InboxProjection<'a, F, E> {
    pub fn new(feed: F, emitter: &'a E) -> Self {
        Self { feed, emitter }
    }

    /// Projects one batch; returns how many events it read.
    pub async fn project(
        mut self,
        batch: i64,
        settle_secs: i64,
        now: DateTime<Utc>,
    ) -> Result<usize, CoreError> {
        let cursor = self.feed.lock_cursor(INBOX_PROJECTION).await?;
        let events = self.feed.events_after(cursor, settle_secs, batch).await?;
        let Some(last) = events.last().map(|event| event.id) else {
            return Ok(0);
        };
        for envelope in &events {
            let organization_id = OrganizationId(envelope.organization_id);
            for notification in self.notifications(envelope).await? {
                let item = InboxItem::project(organization_id, envelope.id, notification, now);
                if self.feed.insert(&item).await? {
                    self.emitter.emit(&InboxItemCreated {
                        item_id: item.id.0,
                        organization_id: organization_id.0,
                        member_subject: item.member_subject.clone(),
                    })?;
                }
            }
        }
        self.feed.advance(INBOX_PROJECTION, last).await?;
        Ok(events.len())
    }

    async fn notifications(
        &mut self,
        envelope: &EventEnvelope,
    ) -> Result<Vec<Notification>, CoreError> {
        let organization_id = OrganizationId(envelope.organization_id);
        let found = match envelope.kind.as_str() {
            ApprovalRequested::KIND => match open::<ApprovalRequested>(envelope) {
                Some(event) => self.approval(organization_id, &event).await?,
                None => None,
            },
            RunFinished::KIND => match open::<RunFinished>(envelope) {
                Some(event) => self
                    .feed
                    .run(organization_id, event.run_id)
                    .await?
                    .and_then(|run| run_finished(&event, &run)),
                None => None,
            },
            CardUpdated::KIND => match open::<CardUpdated>(envelope) {
                Some(event) if event.assigned_to.is_some() => {
                    let agent = match event.author.actor.as_deref() {
                        Some(actor) => self.agent(organization_id, actor).await?,
                        None => None,
                    };
                    self.feed
                        .card(organization_id, event.card_id)
                        .await?
                        .and_then(|card| card_assigned(&event, &card, agent.as_deref()))
                }
                _ => None,
            },
            CardCommented::KIND => match open::<CardCommented>(envelope) {
                Some(event) => self.comment(organization_id, &event).await?,
                None => None,
            },
            SourceEventProcessed::KIND => match open::<SourceEventProcessed>(envelope) {
                Some(event) if event.status == "failed" => self
                    .feed
                    .source(organization_id, event.source_id)
                    .await?
                    .and_then(|source| source_failed(&event, &source)),
                _ => None,
            },
            RoutineRunSkipped::KIND => match open::<RoutineRunSkipped>(envelope) {
                Some(event) if event.status == "failed" => self
                    .feed
                    .routine(organization_id, event.routine_id)
                    .await?
                    .and_then(|routine| routine_failed(&event, &routine)),
                _ => None,
            },
            _ => None,
        };
        Ok(found.into_iter().collect())
    }

    async fn approval(
        &mut self,
        organization_id: OrganizationId,
        event: &ApprovalRequested,
    ) -> Result<Option<Notification>, CoreError> {
        let recipient = match event.run_id {
            Some(run_id) => self
                .feed
                .run(organization_id, run_id)
                .await?
                .map(|run| run.initiated_by),
            None => None,
        };
        let recipient = match (recipient, event.card_id) {
            (Some(recipient), _) => Some(recipient),
            (None, Some(card_id)) => self
                .feed
                .card(organization_id, card_id)
                .await?
                .map(|card| card.created_by),
            (None, None) => None,
        };
        let Some(recipient) = recipient else {
            return Ok(None);
        };
        let title = self
            .feed
            .approval_title(organization_id, event.approval_id)
            .await?
            .unwrap_or_else(|| "Validation demandée".into());
        Ok(Some(approval_requested(event, recipient, title)))
    }

    async fn comment(
        &mut self,
        organization_id: OrganizationId,
        event: &CardCommented,
    ) -> Result<Option<Notification>, CoreError> {
        let Some(actor) = event.author.actor.as_deref().filter(|a| is_agent_actor(a)) else {
            return Ok(None);
        };
        let Some(card) = self.feed.card(organization_id, event.card_id).await? else {
            return Ok(None);
        };
        let run = match event.run_id {
            Some(run_id) => self.feed.run(organization_id, run_id).await?,
            None => None,
        };
        let agent = self
            .agent(organization_id, actor)
            .await?
            .unwrap_or_else(|| actor.trim_start_matches(AGENT_ACTOR_PREFIX).to_owned());
        Ok(agent_commented(event, &card, &agent, run.as_ref()))
    }

    async fn agent(
        &mut self,
        organization_id: OrganizationId,
        actor: &str,
    ) -> Result<Option<String>, CoreError> {
        match actor.strip_prefix(AGENT_ACTOR_PREFIX) {
            Some(slug) => self.feed.agent_name(organization_id, slug).await,
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use capitainerie_events::card::CardAuthor;
    use uuid::Uuid;

    use super::*;
    use crate::domain::inbox::ports::MockInboxFeed;

    fn run(agent: Option<&str>) -> RunFacts {
        RunFacts {
            initiated_by: "user-1".into(),
            agent_name: agent.map(str::to_owned),
            card_id: Some(Uuid::now_v7()),
            card_title: Some("Revue sécurité".into()),
        }
    }

    fn finished(status: &str) -> RunFinished {
        RunFinished {
            run_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            status: status.into(),
            failure: Some("délai dépassé".into()),
        }
    }

    fn card() -> CardFacts {
        CardFacts {
            created_by: "user-1".into(),
            title: "Facture".into(),
        }
    }

    fn author(subject: &str, actor: Option<&str>) -> CardAuthor {
        CardAuthor {
            subject: subject.into(),
            actor: actor.map(str::to_owned),
        }
    }

    #[test]
    fn an_agent_task_tells_its_initiator_how_it_ended() {
        let success = run_finished(&finished("succeeded"), &run(Some("Iris"))).unwrap();
        assert_eq!(success.member_subject, "user-1");
        assert_eq!(success.kind, InboxKind::Run);
        assert_eq!(success.title, "Iris a terminé « Revue sécurité »");
        let failure = run_finished(&finished("failed"), &run(Some("Iris"))).unwrap();
        assert_eq!(failure.title, "Iris a échoué sur « Revue sécurité »");
        assert_eq!(failure.body, "délai dépassé");
        assert!(failure.link.unwrap().starts_with("/tasks/"));
        assert!(run_finished(&finished("cancelled"), &run(Some("Iris"))).is_none());
        assert!(run_finished(&finished("succeeded"), &run(None)).is_none());
    }

    #[test]
    fn an_assignment_reaches_the_assignee_unless_they_did_it() {
        let mut event = CardUpdated {
            card_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            version: 2,
            run_id: None,
            author: author("user-1", None),
            assigned_to: Some("user-2".into()),
        };
        let notification = card_assigned(&event, &card(), None).unwrap();
        assert_eq!(notification.member_subject, "user-2");
        assert_eq!(notification.title, "Carte assignée : « Facture »");
        event.assigned_to = Some("user-1".into());
        assert!(card_assigned(&event, &card(), None).is_none());
        event.author = author("user-1", Some("agent:iris"));
        assert_eq!(
            card_assigned(&event, &card(), Some("Iris")).unwrap().body,
            "par Iris"
        );
        event.assigned_to = None;
        assert!(card_assigned(&event, &card(), None).is_none());
    }

    #[test]
    fn an_agent_comment_reaches_the_card_creator_once() {
        let event = CardCommented {
            card_id: Uuid::now_v7(),
            comment_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            run_id: None,
            author: author("user-9", Some("agent:iris")),
        };
        let notification = agent_commented(&event, &card(), "Iris", None).unwrap();
        assert_eq!(notification.member_subject, "user-1");
        assert_eq!(notification.title, "Iris a commenté « Facture »");
        assert!(agent_commented(&event, &card(), "Iris", Some(&run(Some("Iris")))).is_none());
        let human = CardCommented {
            author: author("user-9", None),
            ..event.clone()
        };
        assert!(agent_commented(&human, &card(), "Iris", None).is_none());
        let captain = CardCommented {
            author: author("user-9", Some("capitainerie-captain")),
            ..event
        };
        assert!(agent_commented(&captain, &card(), "Iris", None).is_none());
    }

    #[test]
    fn only_failures_of_sources_and_routines_are_notified() {
        let owner = OwnedFacts {
            created_by: "user-3".into(),
            name: "Relance".into(),
        };
        let mut processed = SourceEventProcessed {
            event_id: Uuid::now_v7(),
            source_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            status: "failed".into(),
            card_id: None,
            run_id: None,
        };
        assert_eq!(
            source_failed(&processed, &owner).unwrap().member_subject,
            "user-3"
        );
        processed.status = "ignored".into();
        assert!(source_failed(&processed, &owner).is_none());
        let mut skipped = RoutineRunSkipped {
            routine_run_id: Uuid::now_v7(),
            routine_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            origin: "planning".into(),
            scheduled_for: Utc::now(),
            status: "failed".into(),
            reason: "agent archivé".into(),
        };
        let notification = routine_failed(&skipped, &owner).unwrap();
        assert_eq!(notification.title, "Routine « Relance » en échec");
        assert_eq!(notification.body, "agent archivé");
        skipped.status = "skipped".into();
        assert!(routine_failed(&skipped, &owner).is_none());
    }

    #[derive(Default)]
    struct RecordingEmitter(Mutex<Vec<String>>);

    impl EventEmitter for RecordingEmitter {
        fn emit<Ev: DomainEvent>(&self, _event: &Ev) -> Result<(), CoreError> {
            self.0.lock().unwrap().push(Ev::KIND.to_owned());
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_batch_inserts_announces_and_advances_the_cursor() {
        let event = finished("failed");
        let envelope = EventEnvelope::wrap(&event, Utc::now(), None).unwrap();
        let other = EventEnvelope::wrap(
            &ApprovalRequested {
                approval_id: Uuid::now_v7(),
                organization_id: Uuid::now_v7(),
                kind: "tool_call".into(),
                requested_by_agent_id: None,
                card_id: None,
                run_id: None,
            },
            Utc::now(),
            None,
        )
        .unwrap();
        let last = other.id;
        let mut feed = MockInboxFeed::new();
        feed.expect_lock_cursor()
            .returning(|_| Box::pin(async { Ok(Uuid::nil()) }));
        feed.expect_events_after().returning(move |_, _, _| {
            let events = vec![envelope.clone(), other.clone()];
            Box::pin(async move { Ok(events) })
        });
        feed.expect_run()
            .returning(|_, _| Box::pin(async { Ok(Some(run(Some("Iris")))) }));
        feed.expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(true) }));
        feed.expect_advance()
            .times(1)
            .withf(move |name, to| name == INBOX_PROJECTION && *to == last)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let emitter = RecordingEmitter::default();
        let read = InboxProjection::new(feed, &emitter)
            .project(10, 0, Utc::now())
            .await
            .unwrap();
        assert_eq!(read, 2);
        assert_eq!(*emitter.0.lock().unwrap(), ["inbox.item_created"]);
    }
}
