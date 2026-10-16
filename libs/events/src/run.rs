use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainEvent, Topic};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunQueued {
    pub run_id: Uuid,
    pub organization_id: Uuid,
    pub transcript_id: Uuid,
    pub initiated_by: String,
    pub pipeline: Vec<String>,
}

impl DomainEvent for RunQueued {
    const KIND: &'static str = "run.queued";
    const TOPIC: Topic = Topic::Runs;

    fn organization_id(&self) -> Uuid {
        self.organization_id
    }

    fn partition_key(&self) -> Uuid {
        self.run_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStarted {
    pub run_id: Uuid,
    pub organization_id: Uuid,
    /// True when a worker picks up a run another one left unfinished.
    pub resumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepFinished {
    pub run_id: Uuid,
    pub organization_id: Uuid,
    pub seq: i32,
    pub kind: String,
    pub status: String,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
    pub latency_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunFinished {
    pub run_id: Uuid,
    pub organization_id: Uuid,
    pub status: String,
    pub failure: Option<String>,
}

macro_rules! run_event {
    ($event:ty, $kind:literal) => {
        impl DomainEvent for $event {
            const KIND: &'static str = $kind;
            const TOPIC: Topic = Topic::Runs;

            fn organization_id(&self) -> Uuid {
                self.organization_id
            }

            fn partition_key(&self) -> Uuid {
                self.run_id
            }
        }
    };
}

run_event!(RunStarted, "run.started");
run_event!(StepFinished, "run.step_finished");
run_event!(RunFinished, "run.finished");
