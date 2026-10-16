pub mod ports;

use capitainerie_common::{CoreError, OrganizationId};
use uuid::Uuid;

use self::ports::Authorizer;

/// AuthZEN-shaped: who, does what, on which resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequest {
    pub subject: Subject,
    pub action: Action,
    pub resource: Resource,
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
        }
    }
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::Run => "run",
        }
    }
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
