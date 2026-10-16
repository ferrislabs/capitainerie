pub mod ports;

use capitainerie_common::{CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use self::ports::Authorizer;
use crate::domain::card::Column;

/// AuthZEN-shaped: who, does what, on which resource, in which run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequest {
    pub subject: Subject,
    pub action: Action,
    pub resource: Resource,
    /// The run on whose behalf the request is made, so its timeline can show
    /// the decision.
    pub run_id: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// FerrisKey subject of the human the request is made for.
    pub id: String,
    pub organization_id: OrganizationId,
    /// Client id of the agent acting for that human, when one is.
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TranscriptCreate,
    RunRead,
    CardCreate,
    CardRead,
    CardUpdate { column: Column },
    CardComment,
    CardMove { from: Column, to: Column },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub kind: ResourceKind,
    pub id: Option<Uuid>,
    pub organization_id: OrganizationId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Transcript,
    Run,
    Card,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny { reason: String },
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TranscriptCreate => "transcript.create",
            Self::RunRead => "run.read",
            Self::CardCreate => "card.create",
            Self::CardRead => "card.read",
            Self::CardUpdate { .. } => "card.update",
            Self::CardComment => "card.comment",
            Self::CardMove { .. } => "card.move",
        }
    }
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::Run => "run",
            Self::Card => "card",
        }
    }
}

impl AccessRequest {
    pub fn new(subject: Subject, action: Action, kind: ResourceKind, id: Option<Uuid>) -> Self {
        let organization_id = subject.organization_id;
        Self {
            subject,
            action,
            resource: Resource {
                kind,
                id,
                organization_id,
            },
            run_id: None,
        }
    }

    pub fn in_run(mut self, run_id: Option<RunId>) -> Self {
        self.run_id = run_id;
        self
    }
}

/// A past decision of the policy decision point, as audited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    pub subject: String,
    pub actor: Option<String>,
    pub action: String,
    pub resource_kind: String,
    pub resource_id: Option<Uuid>,
    pub allowed: bool,
    pub reason: Option<String>,
    pub decided_at: DateTime<Utc>,
}

/// Asks the policy decision point and turns a denial into `Forbidden`.
pub async fn require<A: Authorizer>(
    authorizer: &A,
    request: AccessRequest,
) -> Result<(), CoreError> {
    match authorizer.evaluate(&request).await? {
        Decision::Allow => Ok(()),
        Decision::Deny { reason } => Err(CoreError::Forbidden {
            reason: Some(reason),
        }),
    }
}
