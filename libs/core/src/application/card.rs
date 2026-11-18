use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::Subject,
        card::{
            Card, CardComment, Column,
            attributes::{Attributes, CardFilter},
            detail::{CardDetail, CardDetailService},
            edit::{CardEditService, EditCard},
            link::{CardLinkService, LinkCard},
            relations::{check_agent, check_parent},
            service::{CardService, CreateCard, CreateTask, MoveCard, UpdateCard},
        },
        client::links::Change,
        member::{Member, service::MemberService},
    },
    infrastructure::{
        card::postgres::{PgCardCommentRepository, PgCardRepository},
        transcript::postgres::PgTranscriptRepository,
    },
};

impl CapitainerieUseCase {
    #[transactional(card, card_comment, transcript, emitter, authz)]
    pub async fn create_card(
        &self,
        subject: Subject,
        command: CreateCard,
    ) -> Result<Card, CoreError> {
        CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .create(subject, command)
        .await
    }

    #[transactional(card, card_comment, transcript, emitter, authz)]
    pub async fn update_card(
        &self,
        subject: Subject,
        id: CardId,
        command: UpdateCard,
    ) -> Result<Card, CoreError> {
        CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .update(subject, id, command)
        .await
    }

    #[transactional(card, card_comment, transcript, emitter, authz)]
    pub async fn comment_card(
        &self,
        subject: Subject,
        id: CardId,
        body: String,
        run_id: Option<RunId>,
    ) -> Result<CardComment, CoreError> {
        CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .comment(subject, id, body, run_id)
        .await
    }

    #[transactional(card, card_comment, transcript, emitter, authz)]
    pub async fn move_card(
        &self,
        subject: Subject,
        id: CardId,
        command: MoveCard,
    ) -> Result<Card, CoreError> {
        CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .move_card(subject, id, command)
        .await
    }

    #[transactional(card, card_comment, transcript, emitter, authz)]
    pub async fn list_cards(
        &self,
        subject: Subject,
        filter: CardFilter,
    ) -> Result<Vec<Card>, CoreError> {
        CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .list(subject, &filter)
        .await
    }

    #[transactional(card, card_comment, run, authz)]
    pub async fn card_detail(&self, subject: Subject, id: CardId) -> Result<CardDetail, CoreError> {
        CardDetailService::new(
            card_repository,
            card_comment_repository,
            run_repository,
            authz,
        )
        .get(subject, id)
        .await
    }
}

impl CapitainerieUseCase {
    #[transactional(card, client, project, member, crew_agent, emitter, authz)]
    pub async fn edit_card(
        &self,
        subject: Subject,
        id: CardId,
        command: EditCard,
    ) -> Result<Card, CoreError> {
        CardEditService::new(
            card_repository,
            client_repository,
            project_repository,
            member_repository,
            crew_agent_repository,
            emitter,
            authz,
        )
        .edit(subject, id, command)
        .await
    }

    /// Creates a card by hand, straight in `todo`: a human has no proposal to
    /// validate. The parent, the agent and the links are all checked in the
    /// same transaction, so a wrong id creates nothing.
    #[transactional(
        card,
        card_comment,
        transcript,
        client,
        project,
        crew_agent,
        emitter,
        authz
    )]
    pub async fn create_task(
        &self,
        subject: Subject,
        command: CreateTask,
    ) -> Result<Card, CoreError> {
        let mut card_repository = card_repository;
        let mut crew_agent_repository = crew_agent_repository;
        if subject.actor.is_some() {
            return Err(CoreError::Forbidden {
                reason: Some("only a human can create a card by hand".into()),
            });
        }
        let organization_id = subject.organization_id;
        if let Some(parent) = command.parent {
            check_parent(&mut card_repository, organization_id, None, parent).await?;
        }
        if let Some(agent) = command.assignee_agent {
            check_agent(&mut crew_agent_repository, organization_id, agent).await?;
        }
        let card = CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .create(
            subject.clone(),
            CreateCard {
                title: command.title,
                description: command.description,
                attributes: Attributes {
                    priority: command.priority,
                    assignee_agent_id: command.assignee_agent,
                    parent_id: command.parent,
                    ..Attributes::default()
                },
                ..CreateCard::default()
            },
        )
        .await?;
        let card = if command.client.is_none() && command.project.is_none() {
            card
        } else {
            CardLinkService::new(
                PgCardRepository::new(&tx),
                client_repository,
                project_repository,
                emitter,
                authz,
            )
            .link(
                subject.clone(),
                card.id,
                LinkCard {
                    expected_version: card.version,
                    client: command.client.map_or(Change::Keep, Change::Set),
                    project: command.project.map_or(Change::Keep, Change::Set),
                },
            )
            .await?
        };
        CardService::new(
            PgCardRepository::new(&tx),
            PgCardCommentRepository::new(&tx),
            PgTranscriptRepository::new(&tx),
            emitter,
            authz,
        )
        .move_card(
            subject,
            card.id,
            MoveCard {
                to: Column::Todo,
                expected_version: card.version,
                run_id: None,
            },
        )
        .await
    }

    #[transactional(member, authz)]
    pub async fn record_member(&self, member: Member) -> Result<(), CoreError> {
        MemberService::new(member_repository, authz)
            .record(member)
            .await
    }

    #[transactional(member, authz)]
    pub async fn list_members(&self, subject: Subject) -> Result<Vec<Member>, CoreError> {
        MemberService::new(member_repository, authz)
            .list(subject)
            .await
    }
}
