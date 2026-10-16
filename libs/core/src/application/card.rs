use capitainerie_common::{CardId, CoreError, RunId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    authorization::Subject,
    card::{
        Card, CardComment,
        service::{CardService, CreateCard, MoveCard, UpdateCard},
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
    pub async fn list_cards(&self, subject: Subject) -> Result<Vec<Card>, CoreError> {
        CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .list(subject)
        .await
    }
}
