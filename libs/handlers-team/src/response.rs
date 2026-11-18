use capitainerie_common::AgentId;
use capitainerie_core::domain::crew::{AgentRevision, CrewAgent};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TeamAgentStatusDto {
    Idle,
    Running,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamAgentResponse {
    pub id: AgentId,
    pub slug: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub model: String,
    pub status: TeamAgentStatusDto,
    pub hue: i16,
    pub reports_to: Option<AgentId>,
    pub archived: bool,
    /// The system prompt.
    pub instructions: String,
    /// Names of the connectors the agent may use.
    pub connector_grants: Vec<String>,
    /// The console's settings of the agent, a JSON object.
    #[schema(value_type = Object)]
    pub config: Value,
    pub skills: Vec<String>,
    pub paused: bool,
    /// The latest entry of the agent's history; every change increments it.
    pub revision: i32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamAgentListResponse {
    pub items: Vec<TeamAgentResponse>,
}

impl From<CrewAgent> for TeamAgentResponse {
    fn from(agent: CrewAgent) -> Self {
        Self {
            id: agent.id,
            slug: agent.slug.as_str().to_owned(),
            name: agent.name,
            role: agent.role,
            description: agent.description,
            model: agent.model,
            status: match agent.status {
                capitainerie_core::domain::crew::AgentStatus::Idle => TeamAgentStatusDto::Idle,
                capitainerie_core::domain::crew::AgentStatus::Running => {
                    TeamAgentStatusDto::Running
                }
            },
            hue: agent.hue.get(),
            reports_to: agent.reports_to,
            archived: agent.archived,
            instructions: agent.instructions,
            connector_grants: agent.connector_grants.as_slice().to_vec(),
            config: agent.config.to_value(),
            skills: agent.skills.as_slice().to_vec(),
            paused: agent.paused,
            revision: agent.revision,
            created_by: agent.created_by,
            created_at: agent.created_at,
            updated_at: agent.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamAgentRevisionResponse {
    pub revision: i32,
    /// What the change was about; may be empty.
    pub changed: String,
    /// FerrisKey subject of the member who made the change.
    pub author: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamAgentRevisionListResponse {
    /// Newest first.
    pub items: Vec<TeamAgentRevisionResponse>,
}

impl From<AgentRevision> for TeamAgentRevisionResponse {
    fn from(revision: AgentRevision) -> Self {
        Self {
            revision: revision.revision,
            changed: revision.changed,
            author: revision.author,
            created_at: revision.created_at,
        }
    }
}
