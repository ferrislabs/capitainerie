use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainEvent, Topic};

/// Who changed a card: the human, and the agent acting for them if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardAuthor {
    pub subject: String,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardProposed {
    pub card_id: Uuid,
    pub organization_id: Uuid,
    pub title: String,
    pub run_id: Option<Uuid>,
    pub transcript_id: Option<Uuid>,
    pub author: CardAuthor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardUpdated {
    pub card_id: Uuid,
    pub organization_id: Uuid,
    pub version: i32,
    pub run_id: Option<Uuid>,
    pub author: CardAuthor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardMoved {
    pub card_id: Uuid,
    pub organization_id: Uuid,
    pub from: String,
    pub to: String,
    pub version: i32,
    pub run_id: Option<Uuid>,
    pub author: CardAuthor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardCommented {
    pub card_id: Uuid,
    pub comment_id: Uuid,
    pub organization_id: Uuid,
    pub run_id: Option<Uuid>,
    pub author: CardAuthor,
}

macro_rules! card_event {
    ($event:ty, $kind:literal) => {
        impl DomainEvent for $event {
            const KIND: &'static str = $kind;
            const TOPIC: Topic = Topic::Cards;

            fn organization_id(&self) -> Uuid {
                self.organization_id
            }

            fn partition_key(&self) -> Uuid {
                self.card_id
            }
        }
    };
}

card_event!(CardProposed, "card.proposed");
card_event!(CardUpdated, "card.updated");
card_event!(CardMoved, "card.moved");
card_event!(CardCommented, "card.commented");
