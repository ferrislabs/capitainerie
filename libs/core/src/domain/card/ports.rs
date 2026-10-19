use capitainerie_common::{CardId, CoreError, OrganizationId};

use super::{Card, CardComment};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait CardRepository: Send {
    fn insert(&mut self, card: &Card) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: CardId,
    ) -> impl Future<Output = Result<Option<Card>, CoreError>> + Send;

    /// Like [`Self::find`], without locking the row: for reads.
    fn get(
        &mut self,
        organization_id: OrganizationId,
        id: CardId,
    ) -> impl Future<Output = Result<Option<Card>, CoreError>> + Send;

    /// Saves `card`, whose version was already incremented, only if the stored
    /// version is still `card.version - 1`; `Conflict` otherwise.
    fn update(&mut self, card: &Card) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn list(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<Vec<Card>, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait CardCommentRepository: Send {
    fn insert(
        &mut self,
        comment: &CardComment,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Oldest first.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> impl Future<Output = Result<Vec<CardComment>, CoreError>> + Send;
}
