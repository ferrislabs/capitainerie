pub mod ports;

use capitainerie_common::{CoreError, OrganizationId, RunId};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use self::ports::Authorizer;
use crate::domain::{card::Column, integration::action::Access};

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
    /// Holds the organization's `capitainerie-admin` realm role.
    pub is_admin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TranscriptCreate,
    TranscriptionCreate,
    DocumentRead,
    RunRead,
    CardCreate,
    CardRead,
    CardUpdate { column: Column },
    CardLink,
    CardEdit,
    CardComment,
    CardMove { from: Column, to: Column },
    ProviderRead,
    ProviderManage,
    ModelsAssign,
    CardExecute,
    GithubManage,
    ClientRead,
    ClientManage,
    MemberRead,
    GithubRead,
    ConversationRead,
    ConversationWrite,
    IntegrationRead,
    IntegrationManage,
    IntegrationCall { access: Access },
    CardIntervene,
    IntegrationRevert,
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
    Transcription,
    Document,
    Run,
    Card,
    Provider,
    ModelAssignments,
    GithubInstallation,
    GithubApp,
    Client,
    Project,
    Member,
    Conversation,
    IntegrationInstance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny { reason: String },
}

pub const READ_ACTIONS: [&str; 6] = [
    "run.read",
    "card.read",
    "provider.read",
    "conversation.read",
    "github.read",
    "integration.read",
];

impl Action {
    pub fn is_read(self) -> bool {
        READ_ACTIONS.contains(&self.as_str())
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TranscriptCreate => "transcript.create",
            Self::TranscriptionCreate => "transcription.create",
            Self::DocumentRead => "document.read",
            Self::RunRead => "run.read",
            Self::CardCreate => "card.create",
            Self::CardRead => "card.read",
            Self::CardUpdate { .. } | Self::CardLink | Self::CardEdit => "card.update",
            Self::CardComment => "card.comment",
            Self::CardMove { .. } => "card.move",
            Self::ProviderRead => "provider.read",
            Self::ProviderManage => "provider.manage",
            Self::ModelsAssign => "models.assign",
            Self::CardExecute => "card.execute",
            Self::GithubManage => "github.manage",
            Self::ClientRead => "client.read",
            Self::ClientManage => "client.manage",
            Self::MemberRead => "member.read",
            Self::GithubRead => "github.read",
            Self::ConversationRead => "conversation.read",
            Self::ConversationWrite => "conversation.write",
            Self::IntegrationRead => "integration.read",
            Self::IntegrationManage => "integration.manage",
            Self::IntegrationCall { .. } => "integration.call",
            Self::CardIntervene => "card.intervene",
            Self::IntegrationRevert => "integration.revert",
        }
    }
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::Transcription => "transcription",
            Self::Document => "document",
            Self::Run => "run",
            Self::Card => "card",
            Self::Provider => "provider",
            Self::ModelAssignments => "model_assignments",
            Self::GithubInstallation => "github_installation",
            Self::GithubApp => "github_app",
            Self::Client => "client",
            Self::Project => "project",
            Self::Member => "member",
            Self::Conversation => "conversation",
            Self::IntegrationInstance => "integration_instance",
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
