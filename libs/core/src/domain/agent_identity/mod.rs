pub mod ports;

use capitainerie_common::{AgentId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use crate::domain::agent::SecretToken;

const CLIENT_PREFIX: &str = "capitainerie-agent-";
const FAILURE_MAX_CHARS: usize = 1000;

/// The FerrisKey client id of an agent. Built from the agent's id, never from
/// its slug: renaming an agent keeps its identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClientId(String);

impl AgentClientId {
    pub fn of(agent_id: AgentId) -> Self {
        Self(format!("{CLIENT_PREFIX}{}", agent_id.0))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityStatus {
    Pending,
    Active,
    Disabled,
}

impl IdentityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }
}

impl TryFrom<&str> for IdentityStatus {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            other => Err(CoreError::Internal(format!(
                "unknown agent identity status {other}"
            ))),
        }
    }
}

/// The client FerrisKey holds for an agent: its internal id, which the admin
/// API addresses, and the secret the agent authenticates with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedClient {
    pub client_uuid: String,
    pub secret: SecretToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityState {
    /// Not in FerrisKey yet, or not complete there. `failure` says why the
    /// last attempt stopped.
    Pending {
        failure: Option<String>,
    },
    Active(ProvisionedClient),
    /// The agent is archived. `client` is `None` when it was archived before
    /// its client existed.
    Disabled {
        client: Option<ProvisionedClient>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    pub organization_id: OrganizationId,
    pub agent_id: AgentId,
    pub client_id: AgentClientId,
    pub state: IdentityState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgentIdentity {
    pub fn pending(organization_id: OrganizationId, agent_id: AgentId, now: DateTime<Utc>) -> Self {
        Self {
            organization_id,
            agent_id,
            client_id: AgentClientId::of(agent_id),
            state: IdentityState::Pending { failure: None },
            created_at: now,
            updated_at: now,
        }
    }

    pub fn status(&self) -> IdentityStatus {
        match self.state {
            IdentityState::Pending { .. } => IdentityStatus::Pending,
            IdentityState::Active(_) => IdentityStatus::Active,
            IdentityState::Disabled { .. } => IdentityStatus::Disabled,
        }
    }

    /// The client the agent acts through, only while the identity is active.
    pub fn credential(&self) -> Option<&ProvisionedClient> {
        match &self.state {
            IdentityState::Active(client) => Some(client),
            _ => None,
        }
    }

    /// Only a pending identity becomes active: a disabled one stays disabled
    /// until its agent comes back.
    #[must_use]
    pub fn activated(self, client: ProvisionedClient, now: DateTime<Utc>) -> Self {
        match self.state {
            IdentityState::Pending { .. } => self.with(IdentityState::Active(client), now),
            _ => self,
        }
    }

    #[must_use]
    pub fn failed(self, reason: &str, now: DateTime<Utc>) -> Self {
        match self.state {
            IdentityState::Pending { .. } => {
                let failure = reason.chars().take(FAILURE_MAX_CHARS).collect();
                self.with(
                    IdentityState::Pending {
                        failure: Some(failure),
                    },
                    now,
                )
            }
            _ => self,
        }
    }

    #[must_use]
    pub fn disabled(self, now: DateTime<Utc>) -> Self {
        let client = match self.state {
            IdentityState::Active(client) => Some(client),
            IdentityState::Pending { .. } => None,
            other => {
                return Self {
                    state: other,
                    ..self
                };
            }
        };
        Self {
            state: IdentityState::Disabled { client },
            updated_at: now,
            ..self
        }
    }

    #[must_use]
    pub fn enabled(self, now: DateTime<Utc>) -> Self {
        let state = match self.state {
            IdentityState::Disabled {
                client: Some(client),
            } => IdentityState::Active(client),
            IdentityState::Disabled { client: None } => IdentityState::Pending { failure: None },
            other => {
                return Self {
                    state: other,
                    ..self
                };
            }
        };
        Self {
            state,
            updated_at: now,
            ..self
        }
    }

    fn with(self, state: IdentityState, now: DateTime<Utc>) -> Self {
        Self {
            state,
            updated_at: now,
            ..self
        }
    }
}

/// Creates the agent's client in FerrisKey, or completes it, and records the
/// outcome. A refusal leaves the identity pending with the reason, for a
/// later attempt: provisioning converges, so retrying is always safe.
pub async fn provision<P: ports::AgentIdentityProvisioner>(
    provisioner: &P,
    issuer: &str,
    identity: AgentIdentity,
    agent_name: &str,
    now: DateTime<Utc>,
) -> AgentIdentity {
    if identity.status() != IdentityStatus::Pending {
        return identity;
    }
    match provisioner
        .provision(issuer, &identity.client_id, agent_name)
        .await
    {
        Ok(client) => identity.activated(client, now),
        Err(error) => identity.failed(&error.to_string(), now),
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{AgentId, OrganizationId};
    use uuid::Uuid;

    use super::{ports::MockAgentIdentityProvisioner, *};

    fn identity() -> AgentIdentity {
        AgentIdentity::pending(
            OrganizationId(Uuid::nil()),
            AgentId(Uuid::from_u128(7)),
            Utc::now(),
        )
    }

    fn client() -> ProvisionedClient {
        ProvisionedClient {
            client_uuid: "fk-uuid".into(),
            secret: SecretToken::new("s3cret"),
        }
    }

    #[test]
    fn the_client_id_comes_from_the_agent_id() {
        assert_eq!(
            AgentClientId::of(AgentId(Uuid::from_u128(7))).as_str(),
            "capitainerie-agent-00000000-0000-0000-0000-000000000007"
        );
    }

    #[test]
    fn archiving_keeps_the_client_and_unarchiving_brings_it_back() {
        let active = identity().activated(client(), Utc::now());
        let disabled = active.clone().disabled(Utc::now());

        assert_eq!(disabled.status(), IdentityStatus::Disabled);
        assert!(disabled.credential().is_none());
        assert_eq!(disabled.enabled(Utc::now()).state, active.state);
    }

    #[test]
    fn an_agent_archived_before_its_client_existed_goes_back_to_pending() {
        let back = identity().disabled(Utc::now()).enabled(Utc::now());

        assert_eq!(back.state, IdentityState::Pending { failure: None });
    }

    #[test]
    fn a_disabled_identity_is_never_activated() {
        let disabled = identity().disabled(Utc::now());

        let after = disabled.clone().activated(client(), Utc::now());

        assert_eq!(after, disabled);
    }

    #[test]
    fn a_long_failure_is_cut() {
        let failed = identity().failed(&"x".repeat(5000), Utc::now());

        let IdentityState::Pending {
            failure: Some(failure),
        } = failed.state
        else {
            panic!("still pending with a failure");
        };
        assert_eq!(failure.chars().count(), FAILURE_MAX_CHARS);
    }

    #[test]
    fn statuses_round_trip() {
        for status in [
            IdentityStatus::Pending,
            IdentityStatus::Active,
            IdentityStatus::Disabled,
        ] {
            assert_eq!(IdentityStatus::try_from(status.as_str()).unwrap(), status);
        }
        assert!(IdentityStatus::try_from("gone").is_err());
    }

    #[tokio::test]
    async fn a_provisioned_client_activates_the_identity() {
        let mut provisioner = MockAgentIdentityProvisioner::new();
        provisioner
            .expect_provision()
            .withf(|issuer, client_id, name| {
                issuer == "http://idp/realms/acme"
                    && client_id.as_str().starts_with("capitainerie-agent-")
                    && name == "Bernard"
            })
            .returning(|_, _, _| Box::pin(async { Ok(client()) }));

        let identity = provision(
            &provisioner,
            "http://idp/realms/acme",
            identity(),
            "Bernard",
            Utc::now(),
        )
        .await;

        assert_eq!(identity.credential(), Some(&client()));
    }

    #[tokio::test]
    async fn a_refusal_keeps_the_identity_pending_with_the_reason() {
        let mut provisioner = MockAgentIdentityProvisioner::new();
        provisioner.expect_provision().returning(|_, _, _| {
            Box::pin(async { Err(CoreError::Internal("ferriskey is down".into())) })
        });

        let identity = provision(
            &provisioner,
            "http://idp",
            identity(),
            "Bernard",
            Utc::now(),
        )
        .await;

        let IdentityState::Pending {
            failure: Some(failure),
        } = identity.state
        else {
            panic!("still pending with a failure");
        };
        assert!(failure.contains("ferriskey is down"));
    }

    #[tokio::test]
    async fn an_identity_that_is_not_pending_is_left_alone() {
        let provisioner = MockAgentIdentityProvisioner::new();
        let active = identity().activated(client(), Utc::now());

        let after = provision(
            &provisioner,
            "http://idp",
            active.clone(),
            "Bernard",
            Utc::now(),
        )
        .await;

        assert_eq!(after, active);
    }
}
