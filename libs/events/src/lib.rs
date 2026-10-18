//! The event envelope every component exchanges through the bus, and the
//! catalogue of event kinds. Depends on nothing in `capitainerie-core`.

pub mod card;
pub mod run;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Runs,
    Cards,
}

impl Topic {
    pub const ALL: [Topic; 2] = [Topic::Runs, Topic::Cards];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Runs => "runs",
            Self::Cards => "cards",
        }
    }
}

pub trait DomainEvent: Serialize + DeserializeOwned {
    const KIND: &'static str;
    const TOPIC: Topic;

    fn organization_id(&self) -> Uuid;

    /// Events sharing a key land on the same partition, so their order holds.
    fn partition_key(&self) -> Uuid;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub kind: String,
    pub topic: Topic,
    pub organization_id: Uuid,
    pub partition_key: Uuid,
    pub occurred_at: DateTime<Utc>,
    /// W3C `traceparent` of the span that produced the event.
    pub traceparent: Option<String>,
    pub payload: serde_json::Value,
}

impl EventEnvelope {
    pub fn wrap<E: DomainEvent>(
        event: &E,
        occurred_at: DateTime<Utc>,
        traceparent: Option<String>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: Uuid::now_v7(),
            kind: E::KIND.to_owned(),
            topic: E::TOPIC,
            organization_id: event.organization_id(),
            partition_key: event.partition_key(),
            occurred_at,
            traceparent,
            payload: serde_json::to_value(event)?,
        })
    }

    /// `None` when the envelope holds another kind of event.
    pub fn open<E: DomainEvent>(&self) -> Option<Result<E, serde_json::Error>> {
        (self.kind == E::KIND).then(|| serde_json::from_value(self.payload.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::RunQueued;

    fn queued() -> RunQueued {
        RunQueued {
            run_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            transcript_id: Some(Uuid::now_v7()),
            card_id: None,
            initiated_by: "user-1".into(),
            pipeline: vec!["extract".into()],
        }
    }

    #[test]
    fn wrap_then_open_round_trips() {
        let event = queued();
        let envelope = EventEnvelope::wrap(&event, Utc::now(), None).unwrap();

        assert_eq!(envelope.kind, "run.queued");
        assert_eq!(envelope.topic, Topic::Runs);
        assert_eq!(envelope.partition_key, event.run_id);
        assert_eq!(envelope.open::<RunQueued>().unwrap().unwrap(), event);
    }

    #[test]
    fn open_ignores_other_kinds() {
        let mut envelope = EventEnvelope::wrap(&queued(), Utc::now(), None).unwrap();
        envelope.kind = "card.proposed".into();
        assert!(envelope.open::<RunQueued>().is_none());
    }
}
