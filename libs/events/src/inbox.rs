use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainEvent, Topic};

/// A notification reached a member's inbox; its content stays in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxItemCreated {
    pub item_id: Uuid,
    pub organization_id: Uuid,
    pub member_subject: String,
}

impl DomainEvent for InboxItemCreated {
    const KIND: &'static str = "inbox.item_created";
    const TOPIC: Topic = Topic::Inbox;

    fn organization_id(&self) -> Uuid {
        self.organization_id
    }

    fn partition_key(&self) -> Uuid {
        self.item_id
    }
}
