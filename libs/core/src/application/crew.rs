use capitainerie_common::{AgentId, CoreError};
use capitainerie_macros::transactional;
use chrono::Utc;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        agent_identity::{AgentIdentity, ports::AgentIdentityRepository},
        authorization::Subject,
        crew::{
            AgentRevision, CrewAgent,
            service::{CreateAgent, CrewService, UpdateAgent},
        },
    },
    infrastructure::agent_identity::postgres::PgAgentIdentityRepository,
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
        let agent = CrewService::new(crew_agent_repository, authz)
            .create(subject, command)
            .await?;
        PgAgentIdentityRepository::new(&tx, self.secrets.as_deref())
            .upsert(&AgentIdentity::pending(
                agent.organization_id,
                agent.id,
                agent.created_at,
            ))
            .await?;
        Ok(agent)
    }

    #[transactional(crew_agent, authz)]
    pub async fn update_crew_agent(
        &self,
        subject: Subject,
        id: AgentId,
        command: UpdateAgent,
    ) -> Result<CrewAgent, CoreError> {
        let archived = command.archived;
        let agent = CrewService::new(crew_agent_repository, authz)
            .update(subject, id, command)
            .await?;
        if archived.is_some() {
            follow_archive(&tx, self.secrets.as_deref(), &agent).await?;
        }
        Ok(agent)
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

/// Disables the agent's identity when it is archived and enables it back when
/// it returns, in the agent's own transaction. FerrisKey follows on the next
/// sweep of the worker.
async fn follow_archive(
    tx: &crate::infrastructure::postgres::SharedTx<'_>,
    secrets: Option<&crate::infrastructure::crypto::SecretCipher>,
    agent: &CrewAgent,
) -> Result<(), CoreError> {
    let mut identities = PgAgentIdentityRepository::new(tx, secrets);
    let now = Utc::now();
    let current = identities
        .find(agent.organization_id, agent.id)
        .await?
        .unwrap_or_else(|| AgentIdentity::pending(agent.organization_id, agent.id, now));
    let next = if agent.archived {
        current.clone().disabled(now)
    } else {
        current.clone().enabled(now)
    };
    if next == current {
        return Ok(());
    }
    identities.upsert(&next).await
}
