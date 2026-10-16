use std::sync::Arc;

use arrow_array::{
    ArrayRef, Int32Array, Int64Array, RecordBatch, StringArray, TimestampMicrosecondArray,
    UInt64Array,
};
use capitainerie_events::{
    EventEnvelope,
    run::{RunFinished, RunQueued, RunStarted, StepFinished},
};
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::{AnalyticsError, schema};

pub const RUN_QUEUED: &str = "run.queued";
pub const RUN_STARTED: &str = "run.started";
pub const STEP_FINISHED: &str = "run.step_finished";
pub const RUN_FINISHED: &str = "run.finished";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord {
    pub event_id: Uuid,
    pub kind: String,
    pub organization_id: Uuid,
    pub run_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub initiated_by: Option<String>,
    pub seq: Option<i32>,
    pub step_kind: Option<String>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_micros: Option<i64>,
    pub latency_ms: Option<i64>,
    pub status: Option<String>,
    pub failure: Option<String>,
}

impl EventRecord {
    fn base(envelope: &EventEnvelope, run_id: Uuid) -> Self {
        Self {
            event_id: envelope.id,
            kind: envelope.kind.clone(),
            organization_id: envelope.organization_id,
            run_id,
            occurred_at: envelope.occurred_at,
            initiated_by: None,
            seq: None,
            step_kind: None,
            agent: None,
            model: None,
            input_tokens: None,
            output_tokens: None,
            cost_micros: None,
            latency_ms: None,
            status: None,
            failure: None,
        }
    }

    pub fn from_envelope(envelope: &EventEnvelope) -> Result<Option<Self>, AnalyticsError> {
        let record = match envelope.kind.as_str() {
            RUN_QUEUED => envelope.open::<RunQueued>().transpose()?.map(|event| Self {
                initiated_by: Some(event.initiated_by),
                ..Self::base(envelope, event.run_id)
            }),
            RUN_STARTED => envelope
                .open::<RunStarted>()
                .transpose()?
                .map(|event| Self::base(envelope, event.run_id)),
            STEP_FINISHED => envelope
                .open::<StepFinished>()
                .transpose()?
                .map(|event| Self {
                    seq: Some(event.seq),
                    step_kind: Some(event.kind),
                    agent: event.agent,
                    model: event.model,
                    input_tokens: Some(event.input_tokens),
                    output_tokens: Some(event.output_tokens),
                    cost_micros: Some(event.cost_micros),
                    latency_ms: event.latency_ms,
                    status: Some(event.status),
                    ..Self::base(envelope, event.run_id)
                }),
            RUN_FINISHED => envelope
                .open::<RunFinished>()
                .transpose()?
                .map(|event| Self {
                    status: Some(event.status),
                    failure: event.failure,
                    ..Self::base(envelope, event.run_id)
                }),
            _ => None,
        };
        Ok(record)
    }

    pub fn date(&self) -> NaiveDate {
        self.occurred_at.date_naive()
    }
}

pub fn to_batch(records: &[&EventRecord]) -> Result<RecordBatch, AnalyticsError> {
    fn required(values: impl Iterator<Item = String>) -> ArrayRef {
        Arc::new(StringArray::from_iter_values(values))
    }
    fn optional<'a>(values: impl Iterator<Item = Option<&'a str>>) -> ArrayRef {
        Arc::new(StringArray::from_iter(values))
    }
    let columns: Vec<ArrayRef> = vec![
        required(records.iter().map(|r| r.event_id.to_string())),
        required(records.iter().map(|r| r.kind.clone())),
        required(records.iter().map(|r| r.organization_id.to_string())),
        required(records.iter().map(|r| r.run_id.to_string())),
        Arc::new(
            TimestampMicrosecondArray::from_iter_values(
                records.iter().map(|r| r.occurred_at.timestamp_micros()),
            )
            .with_timezone("UTC"),
        ),
        optional(records.iter().map(|r| r.initiated_by.as_deref())),
        Arc::new(Int32Array::from_iter(records.iter().map(|r| r.seq))),
        optional(records.iter().map(|r| r.step_kind.as_deref())),
        optional(records.iter().map(|r| r.agent.as_deref())),
        optional(records.iter().map(|r| r.model.as_deref())),
        Arc::new(UInt64Array::from_iter(
            records.iter().map(|r| r.input_tokens),
        )),
        Arc::new(UInt64Array::from_iter(
            records.iter().map(|r| r.output_tokens),
        )),
        Arc::new(Int64Array::from_iter(records.iter().map(|r| r.cost_micros))),
        Arc::new(Int64Array::from_iter(records.iter().map(|r| r.latency_ms))),
        optional(records.iter().map(|r| r.status.as_deref())),
        optional(records.iter().map(|r| r.failure.as_deref())),
    ];
    Ok(RecordBatch::try_new(schema::schema(), columns)?)
}
