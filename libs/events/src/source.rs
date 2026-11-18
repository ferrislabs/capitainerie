use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainEvent, Topic};

/// An external input reached a source; its payload stays in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEventReceived {
    pub event_id: Uuid,
    pub source_id: Uuid,
    pub organization_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEventProcessed {
    pub event_id: Uuid,
    pub source_id: Uuid,
    pub organization_id: Uuid,
    /// `card_created`, `ignored` or `failed`.
    pub status: String,
    pub card_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
}

macro_rules! source_event {
    ($event:ty, $kind:literal) => {
        impl DomainEvent for $event {
            const KIND: &'static str = $kind;
            const TOPIC: Topic = Topic::Sources;

            fn organization_id(&self) -> Uuid {
                self.organization_id
            }

            fn partition_key(&self) -> Uuid {
                self.source_id
            }
        }
    };
}

source_event!(SourceEventReceived, "source.event_received");
source_event!(SourceEventProcessed, "source.event_processed");
