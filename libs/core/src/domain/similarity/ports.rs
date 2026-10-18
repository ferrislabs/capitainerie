use capitainerie_common::{CoreError, OrganizationId};

use super::{CardVector, Embedding, SimilarCard};
use crate::domain::{agent::Usage, provider::ModelTarget};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait Embedder: Send + Sync {
    fn embed(
        &self,
        target: &ModelTarget,
        texts: &[String],
    ) -> impl Future<Output = Result<(Vec<Embedding>, Usage), CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait VectorStore: Send + Sync {
    /// Inserts the card, or replaces the vector already stored for its id.
    fn upsert(
        &self,
        embedding_model: &str,
        card: CardVector,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn holds_cards(
        &self,
        embedding_model: &str,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    /// The cards closest to `embedding`, best first. Always restricted to
    /// `organization_id`, and never returns a rejected card: a rejected card
    /// must not block a new proposal.
    fn nearest(
        &self,
        embedding_model: &str,
        organization_id: OrganizationId,
        embedding: &Embedding,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<SimilarCard>, CoreError>> + Send;
}
