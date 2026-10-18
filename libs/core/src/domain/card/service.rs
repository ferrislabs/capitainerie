use capitainerie_common::{CardCommentId, CardId, CoreError, RunId, TranscriptId};
use capitainerie_events::card::{CardAuthor, CardCommented, CardMoved, CardProposed, CardUpdated};
use chrono::Utc;

use super::{
    Author, COMMENT_MAX_CHARS, Card, CardComment, CardTitle, Column, DESCRIPTION_MAX_CHARS,
    Provenance, bounded_text,
    ports::{CardCommentRepository, CardRepository},
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    event::ports::EventEmitter,
    transcript::ports::TranscriptRepository,
};

#[derive(Debug, Clone)]
pub struct CreateCard {
    pub title: String,
    pub description: String,
    pub provenance: Option<ProvenanceInput>,
    pub run_id: Option<RunId>,
}

#[derive(Debug, Clone)]
pub struct ProvenanceInput {
    pub transcript_id: TranscriptId,
    pub start: u32,
    pub end: u32,
    pub quote: String,
}

#[derive(Debug, Clone)]
pub struct UpdateCard {
    pub title: Option<String>,
    pub description: Option<String>,
    pub expected_version: i32,
    pub run_id: Option<RunId>,
}

#[derive(Debug, Clone)]
pub struct MoveCard {
    pub to: Column,
    pub expected_version: i32,
    pub run_id: Option<RunId>,
}

pub struct CardService<'a, C, M, T, E, A> {
    cards: C,
    comments: M,
    transcripts: T,
    emitter: &'a E,
    authorizer: &'a A,
}

impl<'a, C, M, T, E, A> CardService<'a, C, M, T, E, A>
where
    C: CardRepository,
    M: CardCommentRepository,
    T: TranscriptRepository,
    E: EventEmitter,
    A: Authorizer,
{
    pub fn new(cards: C, comments: M, transcripts: T, emitter: &'a E, authorizer: &'a A) -> Self {
        Self {
            cards,
            comments,
            transcripts,
            emitter,
            authorizer,
        }
    }

    /// Every new card lands in `Proposed`, waiting for a human to accept it.
    pub async fn create(
        mut self,
        subject: Subject,
        command: CreateCard,
    ) -> Result<Card, CoreError> {
        let organization_id = subject.organization_id;
        require(
            self.authorizer,
            AccessRequest::new(
                subject.clone(),
                Action::CardCreate,
                ResourceKind::Card,
                None,
            )
            .in_run(command.run_id),
        )
        .await?;

        let provenance = match command.provenance {
            Some(input) => Some(self.verified_provenance(&subject, input).await?),
            None if subject.actor.is_some() => {
                return Err(CoreError::Invalid(
                    "an agent must cite the transcript passage the card comes from".into(),
                ));
            }
            None => None,
        };
        let now = Utc::now();
        let card = Card {
            id: CardId::new(),
            organization_id,
            title: CardTitle::try_from(command.title)?,
            description: bounded_text(
                command.description,
                DESCRIPTION_MAX_CHARS,
                "description",
                true,
            )?,
            column: Column::Proposed,
            provenance,
            created_by_run: command.run_id,
            created_by: author(&subject),
            version: 1,
            created_at: now,
            updated_at: now,
        };

        self.cards.insert(&card).await?;
        self.emitter.emit(&CardProposed {
            card_id: card.id.0,
            organization_id: organization_id.0,
            title: card.title.as_str().to_owned(),
            run_id: card.created_by_run.map(|id| id.0),
            transcript_id: card.provenance.as_ref().map(|p| p.transcript_id.0),
            author: CardAuthor::from(&card.created_by),
        })?;
        Ok(card)
    }

    pub async fn update(
        mut self,
        subject: Subject,
        id: CardId,
        command: UpdateCard,
    ) -> Result<Card, CoreError> {
        let mut card = self.load(&subject, id).await?;
        require(
            self.authorizer,
            AccessRequest::new(
                subject.clone(),
                Action::CardUpdate {
                    column: card.column,
                },
                ResourceKind::Card,
                Some(id.0),
            )
            .in_run(command.run_id),
        )
        .await?;
        card.ensure_version(command.expected_version)?;
        if command.title.is_none() && command.description.is_none() {
            return Err(CoreError::Invalid(
                "nothing to update: give a title or a description".into(),
            ));
        }
        if let Some(title) = command.title {
            card.title = CardTitle::try_from(title)?;
        }
        if let Some(description) = command.description {
            card.description =
                bounded_text(description, DESCRIPTION_MAX_CHARS, "description", true)?;
        }
        card.touch(Utc::now());

        self.cards.update(&card).await?;
        self.emitter.emit(&CardUpdated {
            card_id: card.id.0,
            organization_id: card.organization_id.0,
            version: card.version,
            run_id: command.run_id.map(|id| id.0),
            author: CardAuthor::from(&author(&subject)),
        })?;
        Ok(card)
    }

    pub async fn comment(
        mut self,
        subject: Subject,
        id: CardId,
        body: String,
        run_id: Option<RunId>,
    ) -> Result<CardComment, CoreError> {
        let card = self.load(&subject, id).await?;
        require(
            self.authorizer,
            AccessRequest::new(
                subject.clone(),
                Action::CardComment,
                ResourceKind::Card,
                Some(id.0),
            )
            .in_run(run_id),
        )
        .await?;
        let comment = CardComment {
            id: CardCommentId::new(),
            card_id: card.id,
            organization_id: card.organization_id,
            author: author(&subject),
            body: bounded_text(body, COMMENT_MAX_CHARS, "comment", false)?,
            created_at: Utc::now(),
        };

        self.comments.insert(&comment).await?;
        self.emitter.emit(&CardCommented {
            card_id: card.id.0,
            comment_id: comment.id.0,
            organization_id: card.organization_id.0,
            run_id: run_id.map(|id| id.0),
            author: CardAuthor::from(&comment.author),
        })?;
        Ok(comment)
    }

    pub async fn move_card(
        mut self,
        subject: Subject,
        id: CardId,
        command: MoveCard,
    ) -> Result<Card, CoreError> {
        let mut card = self.load(&subject, id).await?;
        let from = card.column;
        require(
            self.authorizer,
            AccessRequest::new(
                subject.clone(),
                Action::CardMove {
                    from,
                    to: command.to,
                },
                ResourceKind::Card,
                Some(id.0),
            )
            .in_run(command.run_id),
        )
        .await?;
        card.ensure_version(command.expected_version)?;
        if from == command.to {
            return Err(CoreError::Invalid(format!(
                "the card is already in {}",
                from.as_str()
            )));
        }
        card.column = command.to;
        card.touch(Utc::now());

        self.cards.update(&card).await?;
        self.emitter.emit(&CardMoved {
            card_id: card.id.0,
            organization_id: card.organization_id.0,
            from: from.as_str().to_owned(),
            to: card.column.as_str().to_owned(),
            version: card.version,
            run_id: command.run_id.map(|id| id.0),
            author: CardAuthor::from(&author(&subject)),
        })?;
        Ok(card)
    }

    pub async fn list(mut self, subject: Subject) -> Result<Vec<Card>, CoreError> {
        let organization_id = subject.organization_id;
        require(
            self.authorizer,
            AccessRequest::new(subject, Action::CardRead, ResourceKind::Card, None),
        )
        .await?;
        self.cards.list(organization_id).await
    }

    async fn load(&mut self, subject: &Subject, id: CardId) -> Result<Card, CoreError> {
        self.cards
            .find(subject.organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    async fn verified_provenance(
        &mut self,
        subject: &Subject,
        input: ProvenanceInput,
    ) -> Result<Provenance, CoreError> {
        let transcript = self
            .transcripts
            .find(subject.organization_id, input.transcript_id)
            .await?
            .ok_or_else(|| {
                CoreError::Invalid(format!("unknown transcript {}", input.transcript_id))
            })?;
        Provenance::verified(
            transcript.id,
            input.start,
            input.end,
            input.quote,
            &transcript.body,
        )
    }
}

fn author(subject: &Subject) -> Author {
    Author {
        subject: subject.id.clone(),
        actor: subject.actor.clone(),
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
        card::ports::{MockCardCommentRepository, MockCardRepository},
        transcript::{
            Transcript, TranscriptBody, TranscriptTitle, ports::MockTranscriptRepository,
        },
    };

    #[derive(Default)]
    struct RecordingEmitter(Mutex<Vec<String>>);

    impl EventEmitter for RecordingEmitter {
        fn emit<Ev: DomainEvent>(&self, _event: &Ev) -> Result<(), CoreError> {
            self.0.lock().unwrap().push(Ev::KIND.to_owned());
            Ok(())
        }
    }

    const BODY: &str = "Claire : Malik migre la CI vers ARM avant fin octobre.";

    fn agent(organization_id: OrganizationId) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id,
            actor: Some("capitainerie-agent-writer".into()),
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

    fn transcripts_with(
        organization_id: OrganizationId,
        id: TranscriptId,
    ) -> MockTranscriptRepository {
        let mut transcripts = MockTranscriptRepository::new();
        transcripts.expect_find().returning(move |_, _| {
            let transcript = Transcript {
                id,
                organization_id,
                submitted_by: "user-1".into(),
                title: TranscriptTitle::try_from("Point".to_string()).unwrap(),
                body: TranscriptBody::try_from(BODY.to_string()).unwrap(),
                created_at: Utc::now(),
            };
            Box::pin(async move { Ok(Some(transcript)) })
        });
        transcripts
    }

    fn stored(organization_id: OrganizationId, column: Column) -> Card {
        Card {
            id: CardId::new(),
            organization_id,
            title: CardTitle::try_from("Migrer la CI".to_string()).unwrap(),
            description: String::new(),
            column,
            provenance: None,
            created_by_run: None,
            created_by: Author {
                subject: "user-1".into(),
                actor: None,
            },
            version: 3,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn cards_holding(card: Card) -> MockCardRepository {
        let mut cards = MockCardRepository::new();
        cards.expect_find().returning(move |_, _| {
            let card = card.clone();
            Box::pin(async move { Ok(Some(card)) })
        });
        cards
    }

    #[tokio::test]
    async fn an_agent_proposes_a_card_citing_the_transcript() {
        let organization_id = OrganizationId::new();
        let transcript_id = TranscriptId::new();
        let mut cards = MockCardRepository::new();
        cards
            .expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let emitter = RecordingEmitter::default();
        let authorizer = allow();

        let card = CardService::new(
            cards,
            MockCardCommentRepository::new(),
            transcripts_with(organization_id, transcript_id),
            &emitter,
            &authorizer,
        )
        .create(
            agent(organization_id),
            CreateCard {
                title: "Migrer la CI vers ARM".into(),
                description: "Avant fin octobre".into(),
                provenance: Some(ProvenanceInput {
                    transcript_id,
                    start: 9,
                    end: 35,
                    quote: "Malik migre la CI vers ARM".into(),
                }),
                run_id: Some(RunId::new()),
            },
        )
        .await
        .unwrap();

        assert_eq!(card.column, Column::Proposed);
        assert_eq!(card.version, 1);
        assert_eq!(
            card.created_by.actor.as_deref(),
            Some("capitainerie-agent-writer")
        );
        assert_eq!(*emitter.0.lock().unwrap(), [CardProposed::KIND]);
    }

    #[tokio::test]
    async fn an_agent_cannot_propose_without_provenance() {
        let emitter = RecordingEmitter::default();
        let authorizer = allow();
        let error = CardService::new(
            MockCardRepository::new(),
            MockCardCommentRepository::new(),
            MockTranscriptRepository::new(),
            &emitter,
            &authorizer,
        )
        .create(
            agent(OrganizationId::new()),
            CreateCard {
                title: "Sans source".into(),
                description: String::new(),
                provenance: None,
                run_id: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn a_stale_version_is_a_conflict_and_writes_nothing() {
        let organization_id = OrganizationId::new();
        let card = stored(organization_id, Column::Todo);
        let emitter = RecordingEmitter::default();
        let authorizer = allow();

        let error = CardService::new(
            cards_holding(card.clone()),
            MockCardCommentRepository::new(),
            MockTranscriptRepository::new(),
            &emitter,
            &authorizer,
        )
        .move_card(
            agent(organization_id),
            card.id,
            MoveCard {
                to: Column::Done,
                expected_version: 2,
                run_id: None,
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Conflict(_)));
        assert!(emitter.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn moving_asks_the_pdp_with_both_columns() {
        let organization_id = OrganizationId::new();
        let card = stored(organization_id, Column::Proposed);
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .withf(|request| {
                request.action
                    == Action::CardMove {
                        from: Column::Proposed,
                        to: Column::Todo,
                    }
            })
            .returning(|_| {
                Box::pin(async {
                    Ok(Decision::Deny {
                        reason: "humans only".into(),
                    })
                })
            });
        let emitter = RecordingEmitter::default();

        let error = CardService::new(
            cards_holding(card.clone()),
            MockCardCommentRepository::new(),
            MockTranscriptRepository::new(),
            &emitter,
            &authorizer,
        )
        .move_card(
            agent(organization_id),
            card.id,
            MoveCard {
                to: Column::Todo,
                expected_version: 3,
                run_id: None,
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
