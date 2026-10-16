use std::sync::Mutex;

use capitainerie_common::CoreError;
use capitainerie_events::{DomainEvent, EventEnvelope};
use chrono::Utc;

use crate::domain::event::ports::EventEmitter;

/// Buffers the events of one transaction until `with_tx_emitting` writes them
/// to the outbox, inside that same transaction.
#[derive(Default)]
pub struct TransactionalEventEmitter {
    buffer: Mutex<Vec<EventEnvelope>>,
}

impl TransactionalEventEmitter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn drain(&self) -> Result<Vec<EventEnvelope>, CoreError> {
        self.buffer
            .lock()
            .map(|mut buffer| std::mem::take(&mut *buffer))
            .map_err(|_| CoreError::Internal("event buffer poisoned".into()))
    }
}

impl EventEmitter for TransactionalEventEmitter {
    fn emit<E: DomainEvent>(&self, event: &E) -> Result<(), CoreError> {
        let envelope = EventEnvelope::wrap(
            event,
            Utc::now(),
            capitainerie_telemetry::current_traceparent(),
        )
        .map_err(|error| CoreError::Internal(format!("encoding {}: {error}", E::KIND)))?;
        self.buffer
            .lock()
            .map(|mut buffer| buffer.push(envelope))
            .map_err(|_| CoreError::Internal("event buffer poisoned".into()))
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_events::run::RunQueued;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn drain_empties_the_buffer() {
        let emitter = TransactionalEventEmitter::new();
        let event = RunQueued {
            run_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            transcript_id: Uuid::now_v7(),
            initiated_by: "user-1".into(),
            pipeline: vec![],
        };
        emitter.emit(&event).unwrap();
        emitter.emit(&event).unwrap();

        assert_eq!(emitter.drain().unwrap().len(), 2);
        assert!(emitter.drain().unwrap().is_empty());
    }
}
