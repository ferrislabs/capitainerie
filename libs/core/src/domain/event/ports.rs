use capitainerie_common::CoreError;
use capitainerie_events::{DomainEvent, EventEnvelope};

/// Collects the events of one transaction. They are written to the outbox in
/// that transaction, so an event exists if and only if its transaction committed.
pub trait EventEmitter: Send + Sync {
    fn emit<E: DomainEvent>(&self, event: &E) -> Result<(), CoreError>;
}

/// Transports committed events. Delivery is at least once: consumers dedupe on
/// `EventEnvelope::id`.
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait EventBus: Send + Sync {
    fn publish(
        &self,
        events: &[EventEnvelope],
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}
