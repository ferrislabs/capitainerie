use std::collections::{BTreeSet, HashMap};

use capitainerie_common::CoreError;
use uuid::Uuid;

use super::{
    Actor, ActorKind, ActorRef, AuditEntry, AuditFilter, AuditPage, actor_of, crew_slug,
    ports::{AuditRepository, NamedAgent},
    severity, summary, uuid_in,
};
use crate::domain::authorization::{
    AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require,
};

pub struct AuditService<'a, R, A> {
    events: R,
    authorizer: &'a A,
}

/// The names the entries of one page refer to.
#[derive(Default)]
struct Names {
    members: HashMap<String, String>,
    agents_by_id: HashMap<Uuid, NamedAgent>,
    agents_by_slug: HashMap<String, NamedAgent>,
    run_agents: HashMap<Uuid, NamedAgent>,
    cards: HashMap<Uuid, String>,
}

impl Names {
    fn actor(&self, reference: &ActorRef) -> Actor {
        let agent = |agent: &NamedAgent| Actor {
            kind: ActorKind::Agent,
            id: Some(agent.id.to_string()),
            name: agent.name.clone(),
        };
        match reference {
            ActorRef::Member(subject) => Actor {
                kind: ActorKind::Member,
                id: Some(subject.clone()),
                name: self
                    .members
                    .get(subject)
                    .cloned()
                    .unwrap_or_else(|| subject.clone()),
            },
            ActorRef::AgentActor(actor) => crew_slug(actor)
                .and_then(|slug| self.agents_by_slug.get(slug))
                .map_or_else(
                    || Actor {
                        kind: ActorKind::Agent,
                        id: Some(actor.clone()),
                        name: crew_slug(actor).unwrap_or(actor).to_owned(),
                    },
                    agent,
                ),
            ActorRef::AgentId(id) => self.agents_by_id.get(id).map_or_else(Actor::system, agent),
            ActorRef::RunAgent(run_id) => self
                .run_agents
                .get(run_id)
                .map_or_else(Actor::system, agent),
            ActorRef::System => Actor::system(),
        }
    }
}

impl<'a, R: AuditRepository, A: Authorizer> AuditService<'a, R, A> {
    pub fn new(events: R, authorizer: &'a A) -> Self {
        Self { events, authorizer }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        mut filter: AuditFilter,
    ) -> Result<AuditPage, CoreError> {
        let organization_id = subject.organization_id;
        require(
            self.authorizer,
            AccessRequest::new(subject, Action::AuditRead, ResourceKind::Audit, None),
        )
        .await?;
        filter.check_window()?;
        let limit = filter.limit as usize;
        filter.limit += 1;
        let mut events = self.events.list(organization_id, &filter).await?;
        let next_cursor = (events.len() > limit).then(|| {
            events.truncate(limit);
            events.last().map(|event| event.id)
        });
        let actors: Vec<ActorRef> = events
            .iter()
            .map(|event| actor_of(&event.kind, &event.payload))
            .collect();
        let names = self.names(organization_id, &events, &actors).await?;
        let items = events
            .into_iter()
            .zip(actors)
            .map(|(event, actor)| {
                let title = uuid_in(&event.payload, "card_id")
                    .and_then(|id| names.cards.get(&id))
                    .map(String::as_str);
                AuditEntry {
                    actor: names.actor(&actor),
                    summary: summary(&event.kind, &event.payload, title),
                    severity: severity(&event.kind, &event.payload),
                    event,
                }
            })
            .collect();
        Ok(AuditPage {
            items,
            next_cursor: next_cursor.flatten(),
        })
    }

    async fn names(
        &mut self,
        organization_id: capitainerie_common::OrganizationId,
        events: &[super::AuditEvent],
        actors: &[ActorRef],
    ) -> Result<Names, CoreError> {
        let mut subjects = BTreeSet::new();
        let mut agent_ids = BTreeSet::new();
        let mut slugs = BTreeSet::new();
        let mut run_ids = BTreeSet::new();
        for actor in actors {
            match actor {
                ActorRef::Member(subject) => {
                    subjects.insert(subject.clone());
                }
                ActorRef::AgentActor(actor) => {
                    if let Some(slug) = crew_slug(actor) {
                        slugs.insert(slug.to_owned());
                    }
                }
                ActorRef::AgentId(id) => {
                    agent_ids.insert(*id);
                }
                ActorRef::RunAgent(run_id) => {
                    run_ids.insert(*run_id);
                }
                ActorRef::System => {}
            }
        }
        let card_ids: BTreeSet<Uuid> = events
            .iter()
            .filter_map(|event| uuid_in(&event.payload, "card_id"))
            .collect();

        let mut names = Names::default();
        if !subjects.is_empty() {
            names.members = self
                .events
                .member_names(organization_id, &Vec::from_iter(subjects))
                .await?;
        }
        if !agent_ids.is_empty() {
            names.agents_by_id = self
                .events
                .agents_by_id(organization_id, &Vec::from_iter(agent_ids))
                .await?
                .into_iter()
                .map(|agent| (agent.id, agent))
                .collect();
        }
        if !slugs.is_empty() {
            names.agents_by_slug = self
                .events
                .agents_by_slug(organization_id, &Vec::from_iter(slugs))
                .await?
                .into_iter()
                .map(|agent| (agent.slug.clone(), agent))
                .collect();
        }
        if !run_ids.is_empty() {
            names.run_agents = self
                .events
                .run_agents(organization_id, &Vec::from_iter(run_ids))
                .await?;
        }
        if !card_ids.is_empty() {
            names.cards = self
                .events
                .card_titles(organization_id, &Vec::from_iter(card_ids))
                .await?;
        }
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;
    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::domain::{
        audit::{AuditEvent, Severity, ports::MockAuditRepository},
        authorization::{Decision, ports::MockAuthorizer},
    };

    fn subject(organization_id: OrganizationId) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id,
            actor: None,
            is_admin: false,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    fn event(kind: &str, payload: serde_json::Value) -> AuditEvent {
        AuditEvent {
            id: Uuid::now_v7(),
            topic: kind.split('.').next().unwrap_or_default().to_owned(),
            kind: kind.to_owned(),
            occurred_at: Utc::now(),
            payload,
        }
    }

    #[tokio::test]
    async fn names_are_resolved_and_the_cursor_points_at_the_last_item() {
        let card = Uuid::now_v7();
        let agent = Uuid::now_v7();
        let events = vec![
            event(
                "card.commented",
                json!({"card_id": card, "author": {"subject": "user-1", "actor": "agent:iris"}}),
            ),
            event(
                "card.moved",
                json!({"card_id": card, "to": "done", "author": {"subject": "user-1", "actor": null}}),
            ),
            event("source.event_received", json!({})),
        ];
        let cursor = events[1].id;
        let mut repository = MockAuditRepository::new();
        repository
            .expect_list()
            .withf(|_, filter| filter.limit == 3)
            .returning(move |_, _| {
                let events = events.clone();
                Box::pin(async move { Ok(events) })
            });
        repository.expect_member_names().returning(|_, _| {
            Box::pin(async { Ok(HashMap::from([("user-1".into(), "Alice".into())])) })
        });
        repository.expect_agents_by_slug().returning(move |_, _| {
            Box::pin(async move {
                Ok(vec![NamedAgent {
                    id: agent,
                    slug: "iris".into(),
                    name: "Iris".into(),
                }])
            })
        });
        repository.expect_card_titles().returning(move |_, _| {
            Box::pin(async move { Ok(HashMap::from([(card, "Facture".to_owned())])) })
        });
        let authorizer = allow();
        let page = AuditService::new(repository, &authorizer)
            .list(
                subject(OrganizationId::new()),
                AuditFilter {
                    limit: 2,
                    ..AuditFilter::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_cursor, Some(cursor));
        assert_eq!(page.items[0].actor.name, "Iris");
        assert_eq!(page.items[0].actor.kind, ActorKind::Agent);
        assert_eq!(page.items[0].actor.id, Some(agent.to_string()));
        assert_eq!(page.items[0].summary, "a commenté la carte « Facture »");
        assert_eq!(page.items[1].actor.name, "Alice");
        assert_eq!(page.items[1].severity, Severity::Info);
    }

    #[tokio::test]
    async fn a_denied_caller_reads_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "agents may not".into(),
                })
            })
        });
        let error = AuditService::new(MockAuditRepository::new(), &authorizer)
            .list(subject(OrganizationId::new()), AuditFilter::default())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
