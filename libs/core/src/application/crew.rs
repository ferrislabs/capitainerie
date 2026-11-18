use capitainerie_common::{AgentId, CoreError};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    authorization::Subject,
    crew::{
        AgentRevision, CrewAgent,
        service::{CreateAgent, CrewService, UpdateAgent},
    },
};

impl CapitainerieUseCase {
    #[transactional(crew_agent, authz)]
    pub async fn list_crew(
        &self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<CrewAgent>, CoreError> {
        CrewService::new(crew_agent_repository, authz)
            .list(subject, archived)
            .await
    }

    #[transactional(crew_agent, authz)]
    pub async fn get_crew_agent(
        &self,
        subject: Subject,
        id: AgentId,
    ) -> Result<CrewAgent, CoreError> {
        CrewService::new(crew_agent_repository, authz)
            .get(subject, id)
            .await
    }

    #[transactional(crew_agent, authz)]
    pub async fn create_crew_agent(
        &self,
        subject: Subject,
        command: CreateAgent,
    ) -> Result<CrewAgent, CoreError> {
        CrewService::new(crew_agent_repository, authz)
            .create(subject, command)
            .await
    }

    #[transactional(crew_agent, authz)]
    pub async fn update_crew_agent(
        &self,
        subject: Subject,
        id: AgentId,
        command: UpdateAgent,
    ) -> Result<CrewAgent, CoreError> {
        CrewService::new(crew_agent_repository, authz)
            .update(subject, id, command)
            .await
    }

    #[transactional(crew_agent, authz)]
    pub async fn list_crew_agent_revisions(
        &self,
        subject: Subject,
        id: AgentId,
    ) -> Result<Vec<AgentRevision>, CoreError> {
        CrewService::new(crew_agent_repository, authz)
            .revisions(subject, id)
            .await
    }

    #[transactional(crew_agent, authz)]
    pub async fn restore_crew_agent_revision(
        &self,
        subject: Subject,
        id: AgentId,
        revision: i32,
    ) -> Result<CrewAgent, CoreError> {
        CrewService::new(crew_agent_repository, authz)
            .restore(subject, id, revision)
            .await
    }
}
