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
