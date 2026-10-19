use std::{collections::BTreeSet, time::Duration};

use capitainerie_bus_iggy::IggyEventBus;
use capitainerie_common::{CardId, CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        card::Card,
        provider::{ModelAssignments, ModelTarget},
        similarity::{
            CardVector, card_text,
            ports::{Embedder, VectorStore},
        },
    },
};
use capitainerie_events::{
    DomainEvent, EventEnvelope, Topic,
    card::{CardMoved, CardProposed, CardUpdated},
};
use capitainerie_vector_lancedb::LanceVectorStore;
use tokio::sync::watch;

const CONSUMER_GROUP: &str = "card-indexer";
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const BATCH: u32 = 20;

/// Keeps the vector index in step with the cards: every card ever proposed,
/// edited or moved is embedded and upserted, so the Deduplicate step sees them
/// all. Each organization is indexed with the embedding model it currently
/// chooses, in the index of that model. The group starts at the beginning of
/// the topic, so a fresh index catches up by itself; the backfill covers an
/// index wiped while the group's offset survived, and an organization that has
/// no index yet for its embedding model.
pub struct CardIndexer<E> {
    usecase: CapitainerieUseCase,
    bus: IggyEventBus,
    embedder: E,
    vectors: LanceVectorStore,
    defaults: ModelAssignments,
}

impl<E: Embedder> CardIndexer<E> {
    pub fn new(
        usecase: CapitainerieUseCase,
        bus: IggyEventBus,
        embedder: E,
        vectors: LanceVectorStore,
        defaults: ModelAssignments,
    ) -> Self {
        Self {
            usecase,
            bus,
            embedder,
            vectors,
            defaults,
        }
    }

    pub async fn run(self, mut stopped: watch::Receiver<bool>) {
        if let Err(error) = self.backfill().await {
            tracing::warn!(%error, "backfilling the card index failed");
        }
        let mut poll = tokio::time::interval(POLL_INTERVAL);
        loop {
            tokio::select! {
                _ = stopped.wait_for(|stop| *stop) => break,
                _ = poll.tick() => self.consume().await,
            }
        }
    }

    async fn consume(&self) {
        let envelopes = match self.bus.poll(Topic::Cards, CONSUMER_GROUP, BATCH).await {
            Ok(envelopes) => envelopes,
            Err(error) => {
                tracing::warn!(%error, "polling cards failed");
                return;
            }
        };
        for (organization_id, card_id) in touched_cards(&envelopes) {
            let (organization_id, card_id) = (OrganizationId(organization_id), CardId(card_id));
            if let Err(error) = self.index(organization_id, card_id).await {
                tracing::warn!(%card_id, %error, "indexing a card failed");
            }
        }
    }

    async fn index(
        &self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> Result<(), CoreError> {
        let Some(card) = self
            .usecase
            .card_for_index(organization_id, card_id)
            .await?
        else {
            return Ok(());
        };
        let target = self
            .usecase
            .embedding_target_for(organization_id, &self.defaults)
            .await?;
        self.usecase
            .ensure_indexed(&self.embedder, &self.vectors, &target, organization_id)
            .await?;
        index_cards(
            &self.embedder,
            &self.vectors,
            &target,
            std::slice::from_ref(&card),
        )
        .await
    }

    async fn backfill(&self) -> Result<(), CoreError> {
        for organization_id in self.usecase.organizations_for_index().await? {
            let result = async {
                let target = self
                    .usecase
                    .embedding_target_for(organization_id, &self.defaults)
                    .await?;
                self.usecase
                    .ensure_indexed(&self.embedder, &self.vectors, &target, organization_id)
                    .await
            }
            .await;
            if let Err(error) = result {
                tracing::warn!(%organization_id, %error, "backfilling an organization failed");
            }
        }
        tracing::info!("card index backfilled");
        Ok(())
    }
}

/// The distinct cards the envelopes say changed, in a stable order. Another
/// event kind (a comment) does not change what a card is about.
fn touched_cards(envelopes: &[EventEnvelope]) -> BTreeSet<(uuid::Uuid, uuid::Uuid)> {
    envelopes
        .iter()
        .filter(|envelope| {
            [CardProposed::KIND, CardUpdated::KIND, CardMoved::KIND]
                .contains(&envelope.kind.as_str())
        })
        .map(|envelope| (envelope.organization_id, envelope.partition_key))
        .collect()
}

async fn index_cards<E: Embedder, V: VectorStore>(
    embedder: &E,
    vectors: &V,
    target: &ModelTarget,
    cards: &[Card],
) -> Result<(), CoreError> {
    if cards.is_empty() {
        return Ok(());
    }
    let texts: Vec<String> = cards
        .iter()
        .map(|card| card_text(card.title.as_str(), &card.description))
        .collect();
    let (embeddings, _) = embedder.embed(target, &texts).await?;
    if embeddings.len() != cards.len() {
        return Err(CoreError::Internal(format!(
            "the embedder returned {} vectors for {} cards",
            embeddings.len(),
            cards.len()
        )));
    }
    for ((card, text), embedding) in cards.iter().zip(texts).zip(embeddings) {
        vectors
            .upsert(
                &target.model.model,
                CardVector {
                    card_id: card.id,
                    organization_id: card.organization_id,
                    text,
                    embedding,
                    column: card.column,
                },
            )
            .await?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::{Arc, Mutex};

    use capitainerie_core::domain::{
        agent::Usage,
        card::{Author, CardTitle, Column},
        provider::{ModelRef, ProviderKind},
        similarity::{
            Embedding,
            ports::{MockEmbedder, MockVectorStore},
        },
    };
    use capitainerie_events::card::{CardAuthor, CardCommented};
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn author() -> CardAuthor {
        CardAuthor {
            subject: "u".into(),
            actor: None,
        }
    }

    fn envelope<E: DomainEvent>(event: &E) -> EventEnvelope {
        EventEnvelope::wrap(event, Utc::now(), None).unwrap()
    }

    fn card(title: &str, description: &str, column: Column) -> Card {
        Card {
            id: CardId::new(),
            organization_id: OrganizationId::new(),
            title: CardTitle::try_from(title.to_owned()).unwrap(),
            description: description.into(),
            column,
            provenance: None,
            created_by_run: None,
            created_by: Author {
                subject: "u".into(),
                actor: None,
            },
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            repository_hint: None,
        }
    }

    fn target() -> ModelTarget {
        ModelTarget {
            model: ModelRef {
                provider: ProviderKind::OpenAiCompatible,
                model: "bge-m3".into(),
            },
            access: None,
        }
    }

    #[test]
    fn proposed_updated_and_moved_cards_are_touched_once_and_comments_ignored() {
        let (org, card_id) = (Uuid::now_v7(), Uuid::now_v7());
        let envelopes = [
            envelope(&CardProposed {
                card_id,
                organization_id: org,
                title: "t".into(),
                run_id: None,
                transcript_id: None,
                author: author(),
            }),
            envelope(&CardUpdated {
                card_id,
                organization_id: org,
                version: 2,
                run_id: None,
                author: author(),
            }),
            envelope(&CardMoved {
                card_id,
                organization_id: org,
                from: "proposed".into(),
                to: "todo".into(),
                version: 3,
                run_id: None,
                author: author(),
            }),
            envelope(&CardCommented {
                card_id: Uuid::now_v7(),
                comment_id: Uuid::now_v7(),
                organization_id: org,
                run_id: None,
                author: author(),
            }),
        ];

        let touched = touched_cards(&envelopes);

        assert_eq!(touched.into_iter().collect::<Vec<_>>(), [(org, card_id)]);
    }

    #[tokio::test]
    async fn a_card_is_indexed_under_its_title_and_description_with_its_column() {
        let cards = [card("Titre", "Détails", Column::Rejected)];
        let mut embedder = MockEmbedder::new();
        embedder.expect_embed().times(1).returning(|target, texts| {
            assert_eq!(target.model.model, "bge-m3");
            assert_eq!(texts, ["Titre\nDétails"]);
            Box::pin(std::future::ready(Ok((
                vec![Embedding(vec![1.0, 2.0])],
                Usage::default(),
            ))))
        });
        let stored = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&stored);
        let mut vectors = MockVectorStore::new();
        vectors
            .expect_upsert()
            .times(1)
            .returning(move |model, vector| {
                assert_eq!(model, "bge-m3");
                sink.lock().unwrap().push(vector);
                Box::pin(std::future::ready(Ok(())))
            });

        index_cards(&embedder, &vectors, &target(), &cards)
            .await
            .unwrap();

        let stored = stored.lock().unwrap();
        assert_eq!(stored[0].card_id, cards[0].id);
        assert_eq!(stored[0].organization_id, cards[0].organization_id);
        assert_eq!(stored[0].column, Column::Rejected);
        assert_eq!(stored[0].text, "Titre\nDétails");
        assert_eq!(stored[0].embedding, Embedding(vec![1.0, 2.0]));
    }

    #[tokio::test]
    async fn indexing_nothing_calls_nothing() {
        let embedder = MockEmbedder::new();
        let vectors = MockVectorStore::new();
        index_cards(&embedder, &vectors, &target(), &[])
            .await
            .unwrap();
    }
}
