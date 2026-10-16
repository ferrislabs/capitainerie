use std::time::{Duration, Instant};

use capitainerie_analytics::EventRecord;

#[derive(Debug)]
pub struct Batcher {
    records: Vec<EventRecord>,
    max_events: usize,
    max_age: Duration,
    since: Option<Instant>,
}

impl Batcher {
    pub fn new(max_events: usize, max_age: Duration) -> Self {
        Self {
            records: Vec::new(),
            max_events: max_events.max(1),
            max_age,
            since: None,
        }
    }

    pub fn push(&mut self, record: EventRecord, now: Instant) {
        self.since.get_or_insert(now);
        self.records.push(record);
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_due(&self, now: Instant) -> bool {
        match self.since {
            Some(since) => {
                self.records.len() >= self.max_events || now.duration_since(since) >= self.max_age
            }
            None => false,
        }
    }

    pub fn take(&mut self) -> Vec<EventRecord> {
        self.since = None;
        std::mem::take(&mut self.records)
    }

    pub fn restore(&mut self, mut records: Vec<EventRecord>, now: Instant) {
        records.append(&mut self.records);
        self.records = records;
        self.since = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_analytics::EventRecord;
    use capitainerie_events::{EventEnvelope, run::RunStarted};
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn record() -> EventRecord {
        let event = RunStarted {
            run_id: Uuid::now_v7(),
            organization_id: Uuid::now_v7(),
            resumed: false,
        };
        let envelope = EventEnvelope::wrap(&event, Utc::now(), None).unwrap();
        EventRecord::from_envelope(&envelope).unwrap().unwrap()
    }

    #[test]
    fn nothing_is_due_when_empty() {
        let batcher = Batcher::new(2, Duration::from_secs(1));
        assert!(!batcher.is_due(Instant::now() + Duration::from_secs(60)));
    }

    #[test]
    fn due_once_enough_events_are_buffered() {
        let now = Instant::now();
        let mut batcher = Batcher::new(2, Duration::from_secs(60));
        batcher.push(record(), now);
        assert!(!batcher.is_due(now));
        batcher.push(record(), now);
        assert!(batcher.is_due(now));
    }

    #[test]
    fn due_once_the_oldest_event_is_old_enough() {
        let now = Instant::now();
        let mut batcher = Batcher::new(500, Duration::from_secs(30));
        batcher.push(record(), now);
        assert!(!batcher.is_due(now + Duration::from_secs(29)));
        assert!(batcher.is_due(now + Duration::from_secs(30)));
    }

    #[test]
    fn take_empties_and_restore_keeps_order() {
        let now = Instant::now();
        let mut batcher = Batcher::new(10, Duration::from_secs(30));
        let first = record();
        batcher.push(first.clone(), now);
        let taken = batcher.take();
        assert!(batcher.is_empty());
        assert!(!batcher.is_due(now + Duration::from_secs(60)));

        let second = record();
        batcher.push(second.clone(), now);
        batcher.restore(taken, now);
        assert_eq!(batcher.take(), vec![first, second]);
    }
}
