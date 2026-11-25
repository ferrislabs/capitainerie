pub mod ports;

use capitainerie_common::{AgentId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use crate::domain::agent::SecretToken;

/// Followed by the agent's id, hyphenated and lowercase.
pub const CLIENT_PREFIX: &str = "capitainerie-agent-";
const FAILURE_MAX_CHARS: usize = 1000;

/// The FerrisKey client id of an agent. Built from the agent's id, never from
/// its slug: renaming an agent keeps its identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClientId(String);

impl AgentClientId {
    pub fn of(agent_id: AgentId) -> Self {
        Self(format!("{CLIENT_PREFIX}{}", agent_id.0))
    }

    /// The agent a client id names, `None` for any other client, such as
    /// the shared `capitainerie-agent-writer`.
    pub fn parse(client_id: &str) -> Option<AgentId> {
        let id = client_id.strip_prefix(CLIENT_PREFIX)?;
        let parsed = uuid::Uuid::try_parse(id).ok()?;
        (parsed.hyphenated().to_string() == id).then_some(AgentId(parsed))
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
    /// False while FerrisKey still has to enable or disable the client after
    /// an unarchive or an archive.
    pub synced: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A client FerrisKey has yet to enable or disable after its agent came back
/// or was archived. Read without the secret, which the sweep never needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsyncedClient {
    pub organization_id: OrganizationId,
    pub agent_id: AgentId,
    pub client_uuid: String,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

/// What the console shows of an identity: never the secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentitySummary {
    pub client_id: AgentClientId,
    pub status: IdentityStatus,
    pub failure: Option<String>,
    pub synced: bool,
    pub updated_at: DateTime<Utc>,
}

impl AgentIdentity {
    pub fn pending(organization_id: OrganizationId, agent_id: AgentId, now: DateTime<Utc>) -> Self {
        Self {
            organization_id,
            agent_id,
            client_id: AgentClientId::of(agent_id),
            state: IdentityState::Pending { failure: None },
            synced: true,
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

    /// An existing client still has to be turned off in FerrisKey: the
    /// identity stays out of sync until it is.
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
        let synced = client.is_none();
        Self {
            state: IdentityState::Disabled { client },
            synced,
            updated_at: now,
            ..self
        }
    }

    #[must_use]
    pub fn enabled(self, now: DateTime<Utc>) -> Self {
        let (state, synced) = match self.state {
            IdentityState::Disabled {
                client: Some(client),
            } => (IdentityState::Active(client), false),
            IdentityState::Disabled { client: None } => {
                (IdentityState::Pending { failure: None }, true)
            }
            other => {
                return Self {
                    state: other,
                    ..self
                };
            }
        };
        Self {
            state,
            synced,
            updated_at: now,
            ..self
        }
    }

    pub fn summary(&self) -> IdentitySummary {
        IdentitySummary {
            client_id: self.client_id.clone(),
            status: self.status(),
            failure: match &self.state {
                IdentityState::Pending { failure } => failure.clone(),
                _ => None,
            },
            synced: self.synced,
            updated_at: self.updated_at,
        }
    }

    fn with(self, state: IdentityState, now: DateTime<Utc>) -> Self {
        Self {
            state,
            synced: true,
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

    #[test]
    fn a_client_id_names_its_agent_and_nothing_else_does() {
        let agent = AgentId(uuid::Uuid::now_v7());
        assert_eq!(
            AgentClientId::parse(AgentClientId::of(agent).as_str()),
            Some(agent)
        );
        assert_eq!(AgentClientId::parse("capitainerie-agent-writer"), None);
        assert_eq!(
            AgentClientId::parse(&AgentClientId::of(agent).as_str().to_uppercase()),
            None
        );
        assert_eq!(
            AgentClientId::parse(&format!("capitainerie-agent-{}", agent.0.simple())),
            None
        );
        assert_eq!(AgentClientId::parse(&agent.0.to_string()), None);
    }

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

    #[test]
    fn archiving_a_client_leaves_ferriskey_to_follow() {
        let disabled = identity()
            .activated(client(), Utc::now())
            .disabled(Utc::now());
        assert!(!disabled.synced);

        let back = disabled.enabled(Utc::now());
        assert_eq!(back.status(), IdentityStatus::Active);
        assert!(!back.synced);
    }

    #[test]
    fn archiving_before_the_client_exists_needs_nothing_from_ferriskey() {
        let disabled = identity().disabled(Utc::now());

        assert!(disabled.synced);
        assert!(disabled.enabled(Utc::now()).synced);
    }

    #[test]
    fn a_summary_carries_the_failure_but_no_secret() {
        let failed = identity().failed("idp down", Utc::now());

        let summary = failed.summary();

        assert_eq!(summary.status, IdentityStatus::Pending);
        assert_eq!(summary.failure.as_deref(), Some("idp down"));
        let active = identity().activated(client(), Utc::now()).summary();
        assert!(!format!("{active:?}").contains("s3cret"));
    }
}
