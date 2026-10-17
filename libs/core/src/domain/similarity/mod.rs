pub mod ports;

use capitainerie_common::{CardId, OrganizationId};

use super::{agent::LocatedAction, card::Column};

pub const DEFAULT_DEDUP_THRESHOLD: f32 = 0.75;
pub const DEFAULT_DEDUP_GRAY_FLOOR: f32 = 0.65;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecidedBy {
    #[default]
    Similarity,
    Judge,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DuplicateMatch {
    pub action: usize,
    pub card_id: CardId,
    pub score: f32,
    #[serde(default)]
    pub decided_by: DecidedBy,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JudgedPair {
    pub action: usize,
    pub card_id: CardId,
    pub score: f32,
    pub same_task: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What the Deduplicate step decided for the extracted actions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DedupOutput {
    pub new: Vec<usize>,
    pub duplicates: Vec<DuplicateMatch>,
    #[serde(default)]
    pub judged: Vec<JudgedPair>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Triage<'a> {
    Duplicate(&'a SimilarCard),
    Judge(&'a SimilarCard),
    New,
}

pub fn triage(candidates: &[SimilarCard], threshold: f32, gray_floor: f32) -> Triage<'_> {
    match candidates.iter().max_by(|a, b| a.score.total_cmp(&b.score)) {
        Some(best) if best.score >= threshold => Triage::Duplicate(best),
        Some(best) if best.score >= gray_floor => Triage::Judge(best),
        _ => Triage::New,
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
    fn a_score_at_the_threshold_is_a_duplicate() {
        let at = near(0.75);
        assert_eq!(
            triage(std::slice::from_ref(&at), 0.75, 0.65),
            Triage::Duplicate(&at)
        );
    }

    #[test]
    fn a_score_between_the_floor_and_the_threshold_goes_to_the_judge() {
        let inside = near(0.74);
        let floor = near(0.65);
        assert_eq!(
            triage(std::slice::from_ref(&inside), 0.75, 0.65),
            Triage::Judge(&inside)
        );
        assert_eq!(
            triage(std::slice::from_ref(&floor), 0.75, 0.65),
            Triage::Judge(&floor)
        );
    }

    #[test]
    fn a_score_below_the_floor_is_new() {
        assert_eq!(triage(&[near(0.64)], 0.75, 0.65), Triage::New);
    }

    #[test]
    fn the_best_candidate_wins_whatever_the_order() {
        let low = near(0.7);
        let high = near(0.95);
        let candidates = [low, high.clone()];
        assert_eq!(triage(&candidates, 0.75, 0.65), Triage::Duplicate(&high));
        let candidates = [near(0.8), near(0.7)];
        assert!(matches!(triage(&candidates, 0.9, 0.65), Triage::Judge(card) if card.score == 0.8));
    }

    #[test]
    fn no_candidate_means_new() {
        assert_eq!(triage(&[], 0.75, 0.65), Triage::New);
    }

    #[test]
    fn an_output_stored_before_the_judge_still_reads() {
        let stored = serde_json::json!({
            "new": [1],
            "duplicates": [{ "action": 0, "card_id": CardId::new().0, "score": 0.9 }],
            "embedding_tokens": 20
        });
        let output: DedupOutput = serde_json::from_value(stored).unwrap();
        assert_eq!(output.duplicates[0].decided_by, DecidedBy::Similarity);
        assert!(output.judged.is_empty());
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
