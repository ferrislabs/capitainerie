use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    authorization::Subject,
    card::{
        Card, CardComment,
        attributes::CardFilter,
        detail::{CardDetail, CardDetailService},
        edit::{CardEditService, EditCard},
        service::{CardService, CreateCard, MoveCard, UpdateCard},
    },
    member::{Member, service::MemberService},
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
    #[transactional(card, client, project, member, emitter, authz)]
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
            emitter,
            authz,
        )
        .edit(subject, id, command)
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
