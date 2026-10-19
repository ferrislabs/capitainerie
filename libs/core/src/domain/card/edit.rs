use capitainerie_common::{CardId, ClientId, CoreError, ProjectId};
use capitainerie_events::card::{CardAuthor, CardUpdated};
use chrono::{NaiveDate, Utc};

use super::{
    Author, Card, CardTitle, DESCRIPTION_MAX_CHARS,
    attributes::{AcceptanceCriteria, Criterion, Estimate, Labels, Priority},
    bounded_text,
    link::resolve_links,
    ports::CardRepository,
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    client::{links::Change, ports::ClientRepository},
    event::ports::EventEmitter,
    member::ports::MemberRepository,
    project::ports::ProjectRepository,
};

/// A human's edit of a card. `Keep` leaves a field alone, `Clear` empties it
/// (a title cannot be emptied; a priority goes back to normal).
#[derive(Debug, Clone)]
pub struct EditCard {
    pub expected_version: i32,
    pub title: Change<String>,
    pub description: Change<String>,
    pub acceptance_criteria: Change<Vec<Criterion>>,
    pub priority: Change<Priority>,
    pub due_date: Change<NaiveDate>,
    pub assignee_subject: Change<String>,
    pub labels: Change<Vec<String>>,
    pub estimate: Change<Estimate>,
    pub client: Change<ClientId>,
    pub project: Change<ProjectId>,
}

impl EditCard {
    pub fn new(expected_version: i32) -> Self {
        Self {
            expected_version,
            title: Change::Keep,
            description: Change::Keep,
            acceptance_criteria: Change::Keep,
            priority: Change::Keep,
            due_date: Change::Keep,
            assignee_subject: Change::Keep,
            labels: Change::Keep,
            estimate: Change::Keep,
            client: Change::Keep,
            project: Change::Keep,
        }
    }

    fn changes_nothing(&self) -> bool {
        self.title == Change::Keep
            && self.description == Change::Keep
            && self.acceptance_criteria == Change::Keep
            && self.priority == Change::Keep
            && self.due_date == Change::Keep
            && self.assignee_subject == Change::Keep
            && self.labels == Change::Keep
            && self.estimate == Change::Keep
            && self.client == Change::Keep
            && self.project == Change::Keep
    }
}

pub struct CardEditService<'a, C, K, P, M, E, A> {
    cards: C,
    clients: K,
    projects: P,
    members: M,
    emitter: &'a E,
    authorizer: &'a A,
}

impl<'a, C, K, P, M, E, A> CardEditService<'a, C, K, P, M, E, A>
where
    C: CardRepository,
    K: ClientRepository,
    P: ProjectRepository,
    M: MemberRepository,
    E: EventEmitter,
    A: Authorizer,
{
    pub fn new(
        cards: C,
        clients: K,
        projects: P,
        members: M,
        emitter: &'a E,
        authorizer: &'a A,
    ) -> Self {
        Self {
            cards,
            clients,
            projects,
            members,
            emitter,
            authorizer,
        }
    }

    pub async fn edit(
        mut self,
        subject: Subject,
        id: CardId,
        command: EditCard,
    ) -> Result<Card, CoreError> {
        let organization_id = subject.organization_id;
        let mut card = self
            .cards
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        require(
            self.authorizer,
            AccessRequest::new(
                subject.clone(),
                Action::CardEdit,
                ResourceKind::Card,
                Some(id.0),
            ),
        )
        .await?;
        card.ensure_version(command.expected_version)?;
        if command.changes_nothing() {
            return Err(CoreError::Invalid(
                "nothing to update: give at least one field".into(),
            ));
        }

        let before = card.clone();
        self.apply(&mut card, command).await?;
        if card == before {
            return Ok(card);
        }
        card.touch(Utc::now());
        self.cards.update(&card).await?;
        self.emitter.emit(&CardUpdated {
            card_id: card.id.0,
            organization_id: card.organization_id.0,
            version: card.version,
            run_id: None,
            author: CardAuthor::from(&Author {
                subject: subject.id,
                actor: subject.actor,
            }),
        })?;
        Ok(card)
    }

    async fn apply(&mut self, card: &mut Card, command: EditCard) -> Result<(), CoreError> {
        match command.title {
            Change::Keep => {}
            Change::Clear => {
                return Err(CoreError::Invalid(
                    "the card title cannot be cleared".into(),
                ));
            }
            Change::Set(title) => card.title = CardTitle::try_from(title)?,
        }
        match command.description {
            Change::Keep => {}
            Change::Clear => card.description = String::new(),
            Change::Set(description) => {
                card.description =
                    bounded_text(description, DESCRIPTION_MAX_CHARS, "description", true)?;
            }
        }
        let attributes = &mut card.attributes;
        match command.acceptance_criteria {
            Change::Keep => {}
            Change::Clear => attributes.acceptance_criteria = AcceptanceCriteria::default(),
            Change::Set(criteria) => {
                attributes.acceptance_criteria = AcceptanceCriteria::try_from(criteria)?;
            }
        }
        match command.priority {
            Change::Keep => {}
            Change::Clear => attributes.priority = Priority::default(),
            Change::Set(priority) => attributes.priority = priority,
        }
        match command.due_date {
            Change::Keep => {}
            Change::Clear => attributes.due_date = None,
            Change::Set(date) => attributes.due_date = Some(date),
        }
        match command.labels {
            Change::Keep => {}
            Change::Clear => attributes.labels = Labels::default(),
            Change::Set(labels) => attributes.labels = Labels::try_from(labels)?,
        }
        match command.estimate {
            Change::Keep => {}
            Change::Clear => attributes.estimate = None,
            Change::Set(estimate) => attributes.estimate = Some(estimate),
        }
        match command.assignee_subject {
            Change::Keep => {}
            Change::Clear => attributes.assignee_subject = None,
            Change::Set(assignee) => {
                let assignee = assignee.trim().to_owned();
                self.members
                    .find(card.organization_id, &assignee)
                    .await?
                    .ok_or_else(|| {
                        CoreError::Invalid(format!(
                            "{assignee} is not a member of the organization"
                        ))
                    })?;
                card.attributes.assignee_subject = Some(assignee);
            }
        }
        if command.client != Change::Keep || command.project != Change::Keep {
            card.links = resolve_links(
                &mut self.clients,
                &mut self.projects,
                card,
                command.client,
                command.project,
            )
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use capitainerie_common::OrganizationId;
    use capitainerie_events::DomainEvent;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        card::{Column, ports::MockCardRepository},
        client::ports::MockClientRepository,
        member::{Member, ports::MockMemberRepository},
        project::ports::MockProjectRepository,
    };

    #[derive(Default)]
    struct RecordingEmitter(Mutex<Vec<String>>);

    impl EventEmitter for RecordingEmitter {
        fn emit<Ev: DomainEvent>(&self, _event: &Ev) -> Result<(), CoreError> {
            self.0.lock().unwrap().push(Ev::KIND.to_owned());
            Ok(())
        }
    }

    fn human(organization_id: OrganizationId) -> Subject {
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

    fn stored(organization_id: OrganizationId) -> Card {
        Card {
            id: CardId::new(),
            organization_id,
            title: CardTitle::try_from("Migrer la CI".to_string()).unwrap(),
            description: "Avant fin octobre".into(),
            column: Column::Todo,
            provenance: None,
            created_by_run: None,
            created_by: Author {
                subject: "user-1".into(),
                actor: None,
            },
            version: 3,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            repository_hint: None,
            links: Default::default(),
            attributes: Default::default(),
        }
    }

    fn cards_holding(card: Card, writes: bool) -> MockCardRepository {
        let mut cards = MockCardRepository::new();
        cards.expect_find().returning(move |_, _| {
            let card = card.clone();
            Box::pin(async move { Ok(Some(card)) })
        });
        if writes {
            cards
                .expect_update()
                .times(1)
                .returning(|_| Box::pin(async { Ok(()) }));
        }
        cards
    }

    fn members_knowing(known: &'static str) -> MockMemberRepository {
        let mut members = MockMemberRepository::new();
        members
            .expect_find()
            .returning(move |organization_id, subject| {
                let found = (subject == known).then(|| Member {
                    organization_id,
                    subject: subject.to_owned(),
                    name: "Claire".into(),
                    email: None,
                    last_seen_at: Utc::now(),
                });
                Box::pin(async move { Ok(found) })
            });
        members
    }

    async fn run(
        card: Card,
        writes: bool,
        members: MockMemberRepository,
        command: EditCard,
    ) -> (Result<Card, CoreError>, Vec<String>) {
        let emitter = RecordingEmitter::default();
        let authorizer = allow();
        let organization_id = card.organization_id;
        let id = card.id;
        let result = CardEditService::new(
            cards_holding(card, writes),
            MockClientRepository::new(),
            MockProjectRepository::new(),
            members,
            &emitter,
            &authorizer,
        )
        .edit(human(organization_id), id, command)
        .await;
        let events = emitter.0.lock().unwrap().clone();
        (result, events)
    }

    #[tokio::test]
    async fn an_edit_changes_the_given_fields_bumps_the_version_and_emits_card_updated() {
        let card = stored(OrganizationId::new());
        let mut command = EditCard::new(3);
        command.title = Change::Set("  Migrer la CI vers ARM ".into());
        command.priority = Change::Set(Priority::Urgent);
        command.estimate = Change::Set(Estimate::M);
        command.due_date = Change::Set(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap());
        command.labels = Change::Set(vec!["CI".into(), "ci".into()]);
        command.acceptance_criteria = Change::Set(vec![Criterion {
            text: " Les builds ARM passent ".into(),
            done: true,
        }]);

        let (result, events) = run(card, true, MockMemberRepository::new(), command).await;
        let card = result.unwrap();

        assert_eq!(card.title.as_str(), "Migrer la CI vers ARM");
        assert_eq!(card.description, "Avant fin octobre");
        assert_eq!(card.version, 4);
        assert_eq!(card.attributes.priority, Priority::Urgent);
        assert_eq!(card.attributes.estimate, Some(Estimate::M));
        assert_eq!(card.attributes.labels.as_slice(), ["CI"]);
        assert_eq!(card.attributes.acceptance_criteria.done_count(), 1);
        assert_eq!(events, [CardUpdated::KIND]);
    }

    #[tokio::test]
    async fn clearing_empties_a_field_and_keep_leaves_the_others() {
        let mut card = stored(OrganizationId::new());
        card.attributes.priority = Priority::High;
        card.attributes.estimate = Some(Estimate::L);
        card.attributes.due_date = NaiveDate::from_ymd_opt(2026, 10, 31);
        card.attributes.assignee_subject = Some("user-2".into());
        card.attributes.labels = Labels::try_from(vec!["infra".into()]).unwrap();
        let mut command = EditCard::new(3);
        command.priority = Change::Clear;
        command.estimate = Change::Clear;
        command.due_date = Change::Clear;
        command.assignee_subject = Change::Clear;
        command.description = Change::Clear;

        let (result, _) = run(card, true, MockMemberRepository::new(), command).await;
        let card = result.unwrap();

        assert_eq!(card.attributes.priority, Priority::Normal);
        assert_eq!(card.attributes.estimate, None);
        assert_eq!(card.attributes.due_date, None);
        assert_eq!(card.attributes.assignee_subject, None);
        assert_eq!(card.description, "");
        assert_eq!(card.attributes.labels.as_slice(), ["infra"]);
    }

    #[tokio::test]
    async fn the_title_cannot_be_cleared() {
        let card = stored(OrganizationId::new());
        let mut command = EditCard::new(3);
        command.title = Change::Clear;
        let (result, events) = run(card, false, MockMemberRepository::new(), command).await;
        assert!(matches!(result, Err(CoreError::Invalid(_))));
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn the_assignee_must_be_a_member() {
        let card = stored(OrganizationId::new());
        let mut stranger = EditCard::new(3);
        stranger.assignee_subject = Change::Set("user-9".into());
        let (result, events) = run(card.clone(), false, members_knowing("user-2"), stranger).await;
        assert!(matches!(result, Err(CoreError::Invalid(message)) if message.contains("user-9")));
        assert!(events.is_empty());

        let mut member = EditCard::new(3);
        member.assignee_subject = Change::Set("user-2".into());
        let (result, _) = run(card, true, members_knowing("user-2"), member).await;
        assert_eq!(
            result.unwrap().attributes.assignee_subject.as_deref(),
            Some("user-2")
        );
    }

    #[tokio::test]
    async fn invalid_values_are_refused_and_nothing_is_written() {
        let invalid = |edit: fn(&mut EditCard)| {
            let mut command = EditCard::new(3);
            edit(&mut command);
            command
        };
        for command in [
            invalid(|c| c.title = Change::Set("  ".into())),
            invalid(|c| c.labels = Change::Set(vec!["a,b".into()])),
            invalid(|c| {
                c.acceptance_criteria = Change::Set(vec![Criterion {
                    text: " ".into(),
                    done: false,
                }]);
            }),
        ] {
            let card = stored(OrganizationId::new());
            let (result, events) = run(card, false, MockMemberRepository::new(), command).await;
            assert!(matches!(result, Err(CoreError::Invalid(_))));
            assert!(events.is_empty());
        }
    }

    #[tokio::test]
    async fn a_stale_version_is_a_conflict_and_writes_nothing() {
        let card = stored(OrganizationId::new());
        let mut command = EditCard::new(2);
        command.priority = Change::Set(Priority::High);
        let (result, events) = run(card, false, MockMemberRepository::new(), command).await;
        assert!(matches!(result, Err(CoreError::Conflict(_))));
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn an_edit_that_changes_nothing_writes_nothing() {
        let card = stored(OrganizationId::new());
        let mut same = EditCard::new(3);
        same.priority = Change::Set(Priority::Normal);
        let (result, events) = run(card.clone(), false, MockMemberRepository::new(), same).await;
        assert_eq!(result.unwrap().version, 3);
        assert!(events.is_empty());

        let (result, _) = run(card, false, MockMemberRepository::new(), EditCard::new(3)).await;
        assert!(matches!(result, Err(CoreError::Invalid(_))));
    }

    #[tokio::test]
    async fn editing_asks_the_pdp_for_card_edit() {
        let card = stored(OrganizationId::new());
        let organization_id = card.organization_id;
        let id = card.id;
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .withf(|request| request.action == Action::CardEdit)
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(Decision::Deny {
                        reason: "humans only".into(),
                    })
                })
            });
        let emitter = RecordingEmitter::default();
        let mut command = EditCard::new(3);
        command.priority = Change::Set(Priority::High);

        let error = CardEditService::new(
            cards_holding(card, false),
            MockClientRepository::new(),
            MockProjectRepository::new(),
            MockMemberRepository::new(),
            &emitter,
            &authorizer,
        )
        .edit(human(organization_id), id, command)
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
