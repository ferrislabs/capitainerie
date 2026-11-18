use capitainerie_common::AgentId;
use capitainerie_core::domain::crew::CrewAgent;
use chrono::{DateTime, Utc};
use serde::Serialize;
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
            created_by: agent.created_by,
            created_at: agent.created_at,
            updated_at: agent.updated_at,
        }
    }
}
