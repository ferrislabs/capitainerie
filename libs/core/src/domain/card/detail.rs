use capitainerie_common::{CardId, CoreError};

use super::{
    Card, CardComment,
    ports::{CardCommentRepository, CardRepository},
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    run::{RunOverview, ports::RunRepository},
};

#[derive(Debug, Clone)]
pub struct CardDetail {
    pub card: Card,
    pub comments: Vec<CardComment>,
    /// The run that proposed the card, then its executions, newest first.
    pub runs: Vec<RunOverview>,
}

pub struct CardDetailService<'a, C, M, R, A> {
    cards: C,
    comments: M,
    runs: R,
    authorizer: &'a A,
}

impl<'a, C, M, R, A> CardDetailService<'a, C, M, R, A>
where
    C: CardRepository,
    M: CardCommentRepository,
    R: RunRepository,
    A: Authorizer,
{
    pub fn new(cards: C, comments: M, runs: R, authorizer: &'a A) -> Self {
        Self {
            cards,
            comments,
            runs,
            authorizer,
        }
    }

    pub async fn get(mut self, subject: Subject, id: CardId) -> Result<CardDetail, CoreError> {
        let organization_id = subject.organization_id;
        let card = self
            .cards
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        require(
            self.authorizer,
            AccessRequest::new(subject, Action::CardRead, ResourceKind::Card, Some(id.0)),
        )
        .await?;

        let comments = self.comments.list(organization_id, id).await?;
        let mut runs = Vec::new();
        if let Some(run_id) = card.created_by_run {
            runs.extend(self.runs.find(organization_id, run_id).await?);
        }
        runs.extend(self.runs.list_for_card(organization_id, id).await?);
        Ok(CardDetail {
            card,
            comments,
            runs,
        })
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{CardCommentId, OrganizationId, RunId, TranscriptId};
    use chrono::Utc;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        card::{
            Author, CardTitle, Column,
            ports::{MockCardCommentRepository, MockCardRepository},
        },
        run::{Run, ports::MockRunRepository},
    };

    fn subject(organization_id: OrganizationId) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id,
            actor: None,
            is_admin: false,
        }
    }

    fn card(organization_id: OrganizationId, created_by_run: Option<RunId>) -> Card {
        Card {
            id: CardId::new(),
            organization_id,
            title: CardTitle::try_from("Migrer la CI".to_string()).unwrap(),
            description: String::new(),
            column: Column::Todo,
            provenance: None,
            created_by_run,
            created_by: Author {
                subject: "user-1".into(),
                actor: None,
            },
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn overview(organization_id: OrganizationId, id: RunId) -> RunOverview {
        let mut run = Run::queue(
            organization_id,
            TranscriptId::new(),
            "user-1".into(),
            Run::MEETING_PIPELINE.to_vec(),
            Utc::now(),
        )
        .0;
        run.id = id;
        RunOverview { run, title: None }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    #[tokio::test]
    async fn a_card_of_another_organization_is_not_found() {
        let mut cards = MockCardRepository::new();
        cards
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        let authorizer = allow();

        let error = CardDetailService::new(
            cards,
            MockCardCommentRepository::new(),
            MockRunRepository::new(),
            &authorizer,
        )
        .get(subject(OrganizationId::new()), CardId::new())
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::NotFound));
    }

    #[tokio::test]
    async fn the_proposing_run_comes_before_the_executions() {
        let organization_id = OrganizationId::new();
        let proposing = RunId::new();
        let newest = RunId::new();
        let oldest = RunId::new();
        let stored = card(organization_id, Some(proposing));
        let mut cards = MockCardRepository::new();
        cards.expect_find().returning(move |_, _| {
            let stored = stored.clone();
            Box::pin(async move { Ok(Some(stored)) })
        });
        let mut comments = MockCardCommentRepository::new();
        comments.expect_list().returning(move |_, card_id| {
            let comment = CardComment {
                id: CardCommentId::new(),
                card_id,
                organization_id,
                author: Author {
                    subject: "user-1".into(),
                    actor: None,
                },
                body: "Bien reçu".into(),
                created_at: Utc::now(),
            };
            Box::pin(async move { Ok(vec![comment]) })
        });
        let mut runs = MockRunRepository::new();
        runs.expect_find().returning(move |_, id| {
            let found = overview(organization_id, id);
            Box::pin(async move { Ok(Some(found)) })
        });
        runs.expect_list_for_card().returning(move |_, _| {
            let listed = vec![
                overview(organization_id, newest),
                overview(organization_id, oldest),
            ];
            Box::pin(async move { Ok(listed) })
        });
        let authorizer = allow();

        let detail = CardDetailService::new(cards, comments, runs, &authorizer)
            .get(subject(organization_id), CardId::new())
            .await
            .unwrap();

        let ids: Vec<RunId> = detail.runs.iter().map(|overview| overview.run.id).collect();
        assert_eq!(ids, [proposing, newest, oldest]);
        assert_eq!(detail.comments.len(), 1);
    }

    #[tokio::test]
    async fn reading_asks_the_pdp_for_card_read() {
        let organization_id = OrganizationId::new();
        let stored = card(organization_id, None);
        let mut cards = MockCardRepository::new();
        cards.expect_find().returning(move |_, _| {
            let stored = stored.clone();
            Box::pin(async move { Ok(Some(stored)) })
        });
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .withf(|request| request.action == Action::CardRead)
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(Decision::Deny {
                        reason: "no".into(),
                    })
                })
            });

        let error = CardDetailService::new(
            cards,
            MockCardCommentRepository::new(),
            MockRunRepository::new(),
            &authorizer,
        )
        .get(subject(organization_id), CardId::new())
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
