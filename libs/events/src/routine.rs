use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainEvent, Topic};

/// A routine passage created its card; `run_id` is set when the agent run started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutineRunCreated {
    pub routine_run_id: Uuid,
    pub routine_id: Uuid,
    pub organization_id: Uuid,
    /// `planning`, `manual` or `webhook`.
    pub origin: String,
    pub scheduled_for: DateTime<Utc>,
    pub card_id: Uuid,
    pub run_id: Option<Uuid>,
}

/// A routine passage created no card: skipped by a policy, or failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutineRunSkipped {
    pub routine_run_id: Uuid,
    pub routine_id: Uuid,
    pub organization_id: Uuid,
    pub origin: String,
    pub scheduled_for: DateTime<Utc>,
    /// `skipped` or `failed`.
    pub status: String,
    pub reason: String,
}

macro_rules! routine_event {
    ($event:ty, $kind:literal) => {
        impl DomainEvent for $event {
            const KIND: &'static str = $kind;
            const TOPIC: Topic = Topic::Routines;

            fn organization_id(&self) -> Uuid {
                self.organization_id
            }

            fn partition_key(&self) -> Uuid {
                self.routine_id
            }
        }
    };
}

routine_event!(RoutineRunCreated, "routine.run_created");
routine_event!(RoutineRunSkipped, "routine.run_skipped");
