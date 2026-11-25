use capitainerie_common::{AgentId, CoreError, OrganizationId};
use capitainerie_macros::transactional;
use chrono::{DateTime, Utc};

use super::CapitainerieUseCase;
use crate::{
    domain::{
        agent_identity::{
            AgentClientId, AgentIdentity, IdentityState, IdentityStatus, IdentitySummary,
            UnsyncedClient,
            ports::{AgentIdentityProvisioner, AgentIdentityRepository},
            provision,
        },
        agent_task::actor_of,
        authorization::Subject,
        crew::{ports::CrewRepository, service::CrewService},
        organization::ports::OrganizationRepository,
    },
    infrastructure::agent_identity::postgres::PgAgentIdentityRepository,
};

/// What a sweep did, for the worker's log and its drain loop.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IdentitySweep {
    pub adopted: u64,
    /// Identities picked up: the sweep is full when this reaches twice its
    /// limit, one for each kind.
    pub picked: usize,
    pub activated: usize,
    pub synced: usize,
    pub failed: usize,
}

/// What a sweep has to do, read in one transaction.
struct Backlog {
    adopted: u64,
    /// Each pending identity with its organization's issuer and its agent's name.
    pending: Vec<(AgentIdentity, String, String)>,
    /// Each client to enable or disable with its organization's issuer.
    unsynced: Vec<(UnsyncedClient, String)>,
}

impl CapitainerieUseCase {
    /// The identity of an agent the subject may read. `None` for an agent
    /// the worker has not adopted yet.
    #[transactional(crew_agent, authz)]
    pub async fn crew_agent_identity(
        &self,
        subject: Subject,
        id: AgentId,
    ) -> Result<Option<IdentitySummary>, CoreError> {
        let agent = CrewService::new(crew_agent_repository, authz)
            .get(subject, id)
            .await?;
        PgAgentIdentityRepository::new(&tx, None)
            .summary(agent.organization_id, agent.id)
            .await
    }

    /// The actor a delegated token's client acts as: `agent:<slug>` when it
    /// is the active client of an unarchived agent of the organization.
    /// `None` for any other client, even one FerrisKey still accepts.
    #[transactional(crew_agent)]
    pub async fn active_agent_actor(
        &self,
        organization_id: OrganizationId,
        client_id: &str,
    ) -> Result<Option<String>, CoreError> {
        let Some(agent_id) = AgentClientId::parse(client_id) else {
            return Ok(None);
        };
        let mut crew_agent_repository = crew_agent_repository;
        let Some(agent) = crew_agent_repository
            .find(organization_id, agent_id)
            .await?
            .filter(|agent| !agent.archived)
        else {
            return Ok(None);
        };
        let active = PgAgentIdentityRepository::new(&tx, None)
            .summary(organization_id, agent_id)
            .await?
            .is_some_and(|identity| identity.status == IdentityStatus::Active);
        Ok(active.then(|| actor_of(agent.slug.as_str())))
    }

    /// Adopts the agents without an identity, provisions up to `limit`
    /// pending identities and has FerrisKey follow up to `limit` archives or
    /// unarchives. FerrisKey is called outside any transaction, and the
    /// stored secrets are never opened. An identity changed meanwhile keeps
    /// that change and waits for the next sweep.
    pub async fn sweep_agent_identities<P: AgentIdentityProvisioner>(
        &self,
        provisioner: &P,
        retry_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<IdentitySweep, CoreError> {
        let backlog = self.identity_backlog(retry_before, limit).await?;
        let mut sweep = IdentitySweep {
            adopted: backlog.adopted,
            picked: backlog.pending.len() + backlog.unsynced.len(),
            ..IdentitySweep::default()
        };
        for (identity, issuer, agent_name) in backlog.pending {
            let agent_id = identity.agent_id;
            let expected = identity.updated_at;
            let next = provision(provisioner, &issuer, identity, &agent_name, Utc::now()).await;
            match &next.state {
                IdentityState::Pending {
                    failure: Some(failure),
                } => {
                    tracing::warn!(%agent_id, %failure, "the agent's client was not provisioned");
                    sweep.failed += 1;
                }
                _ => sweep.activated += 1,
            }
            if !self.store_provisioned(&next, expected).await? {
                tracing::info!(%agent_id, "the identity changed during the sweep, the next one takes it");
            }
        }
        for (client, issuer) in backlog.unsynced {
            let agent_id = client.agent_id;
            let toggled = provisioner
                .set_enabled(&issuer, &client.client_uuid, client.enabled)
                .await;
            if let Err(error) = toggled {
                tracing::warn!(%agent_id, enabled = client.enabled, %error, "FerrisKey did not follow the agent's archive state");
                sweep.failed += 1;
                continue;
            }
            sweep.synced += 1;
            if !self.mark_identity_synced(&client).await? {
                tracing::info!(%agent_id, "the identity changed during the sweep, the next one takes it");
            }
        }
        Ok(sweep)
    }

    #[transactional(organization, crew_agent)]
    async fn identity_backlog(
        &self,
        retry_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Backlog, CoreError> {
        let mut organization_repository = organization_repository;
        let mut crew_agent_repository = crew_agent_repository;
        let mut identities = PgAgentIdentityRepository::new(&tx, None);
        let adopted = identities.adopt(Utc::now()).await?;
        let mut issuers = std::collections::HashMap::new();
        let mut pending = Vec::new();
        for identity in identities.pending(retry_before, limit).await? {
            let issuer = issuer_of(
                &mut organization_repository,
                &mut issuers,
                identity.organization_id,
            )
            .await?;
            let agent = crew_agent_repository
                .find(identity.organization_id, identity.agent_id)
                .await?
                .ok_or(CoreError::NotFound)?;
            pending.push((identity, issuer, agent.name));
        }
        let mut unsynced = Vec::new();
        for client in identities.unsynced(limit).await? {
            let issuer = issuer_of(
                &mut organization_repository,
                &mut issuers,
                client.organization_id,
            )
            .await?;
            unsynced.push((client, issuer));
        }
        Ok(Backlog {
            adopted,
            pending,
            unsynced,
        })
    }

    #[transactional()]
    async fn store_provisioned(
        &self,
        identity: &AgentIdentity,
        expected: DateTime<Utc>,
    ) -> Result<bool, CoreError> {
        PgAgentIdentityRepository::new(&tx, self.secrets.as_deref())
            .upsert_unchanged(identity, expected)
            .await
    }

    #[transactional()]
    async fn mark_identity_synced(&self, client: &UnsyncedClient) -> Result<bool, CoreError> {
        PgAgentIdentityRepository::new(&tx, None)
            .mark_synced(client.organization_id, client.agent_id, client.updated_at)
            .await
    }
}

async fn issuer_of<R: OrganizationRepository>(
    organizations: &mut R,
    known: &mut std::collections::HashMap<capitainerie_common::OrganizationId, String>,
    organization_id: capitainerie_common::OrganizationId,
) -> Result<String, CoreError> {
    if let Some(issuer) = known.get(&organization_id) {
        return Ok(issuer.clone());
    }
    let issuer = organizations
        .find(organization_id)
        .await?
        .ok_or(CoreError::NotFound)?
        .issuer_url;
    known.insert(organization_id, issuer.clone());
    Ok(issuer)
}
