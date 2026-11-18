use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainEvent, Topic};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequested {
    pub approval_id: Uuid,
    pub organization_id: Uuid,
    pub kind: String,
    pub requested_by_agent_id: Option<Uuid>,
    pub card_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecided {
    pub approval_id: Uuid,
    pub organization_id: Uuid,
    /// `approved` or `rejected`.
    pub status: String,
    pub decided_by: String,
    pub card_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
}

macro_rules! approval_event {
    ($event:ty, $kind:literal) => {
        impl DomainEvent for $event {
            const KIND: &'static str = $kind;
            const TOPIC: Topic = Topic::Approvals;

            fn organization_id(&self) -> Uuid {
                self.organization_id
            }

            fn partition_key(&self) -> Uuid {
                self.approval_id
            }
        }
    };
}

approval_event!(ApprovalRequested, "approval.requested");
approval_event!(ApprovalDecided, "approval.decided");
