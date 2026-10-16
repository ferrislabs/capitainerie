pub mod ports;

use capitainerie_common::{CardId, OrganizationId};

use super::{agent::LocatedAction, card::Column};

pub const DEFAULT_DEDUP_THRESHOLD: f32 = 0.75;

#[derive(Debug, Clone, PartialEq)]
pub struct Embedding(pub Vec<f32>);

#[derive(Debug, Clone, PartialEq)]
pub struct CardVector {
    pub card_id: CardId,
    pub organization_id: OrganizationId,
    pub text: String,
    pub embedding: Embedding,
    pub column: Column,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimilarCard {
    pub card_id: CardId,
    /// Cosine similarity, 0 to 1.
    pub score: f32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DuplicateMatch {
    pub action: usize,
    pub card_id: CardId,
    pub score: f32,
}

/// What the Deduplicate step decided for the extracted actions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DedupOutput {
    pub new: Vec<usize>,
    pub duplicates: Vec<DuplicateMatch>,
    pub embedding_tokens: u64,
}

/// The text a card is indexed under.
pub fn card_text(title: &str, description: &str) -> String {
    format!("{title}\n{description}")
}

/// The text an extracted action is compared under: the quote is part of it so
/// that a terse title still carries the meeting's wording.
pub fn action_text(located: &LocatedAction) -> String {
    let action = &located.action;
    format!("{}\n{}\n{}", action.title, action.description, action.quote)
}

/// An action is a duplicate of the closest existing card when that card scores
/// at least `threshold`; otherwise it is new.
pub fn classify(neighbours: &[Vec<SimilarCard>], threshold: f32, tokens: u64) -> DedupOutput {
    let mut new = Vec::new();
    let mut duplicates = Vec::new();
    for (action, candidates) in neighbours.iter().enumerate() {
        let best = candidates
            .iter()
            .filter(|candidate| candidate.score >= threshold)
            .max_by(|a, b| a.score.total_cmp(&b.score));
        match best {
            Some(card) => duplicates.push(DuplicateMatch {
                action,
                card_id: card.card_id,
                score: card.score,
            }),
            None => new.push(action),
        }
    }
    DedupOutput {
        new,
        duplicates,
        embedding_tokens: tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent::ExtractedAction;

    fn near(score: f32) -> SimilarCard {
        SimilarCard {
            card_id: CardId::new(),
            score,
            text: String::new(),
        }
    }

    #[test]
    fn a_score_at_the_threshold_is_a_duplicate_and_below_is_new() {
        let at = near(0.86);
        let below = near(0.85);
        let output = classify(&[vec![at.clone()], vec![below]], 0.86, 7);
        assert_eq!(output.new, [1]);
        assert_eq!(
            output.duplicates,
            [DuplicateMatch {
                action: 0,
                card_id: at.card_id,
                score: 0.86
            }]
        );
        assert_eq!(output.embedding_tokens, 7);
    }

    #[test]
    fn the_best_candidate_wins_whatever_the_order() {
        let low = near(0.88);
        let high = near(0.95);
        let output = classify(&[vec![low, high.clone()]], 0.86, 0);
        assert_eq!(output.duplicates[0].card_id, high.card_id);
    }

    #[test]
    fn no_candidate_means_new() {
        let output = classify(&[vec![], vec![]], 0.86, 0);
        assert_eq!(output.new, [0, 1]);
        assert!(output.duplicates.is_empty());
    }

    #[test]
    fn texts_join_the_parts_with_newlines() {
        let located = LocatedAction {
            action: ExtractedAction {
                title: "T".into(),
                description: "D".into(),
                quote: "Q".into(),
                owner: None,
                due: None,
            },
            start: 0,
            end: 1,
        };
        assert_eq!(action_text(&located), "T\nD\nQ");
        assert_eq!(card_text("T", "D"), "T\nD");
    }
}
