mod gateway;
mod oauth;

use std::time::Duration;

use capitainerie_common::CoreError;
use capitainerie_core::domain::integration::ConnectorFailure;

use crate::{NetworkPolicy, guard};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct McpHttp {
    policy: NetworkPolicy,
    client: reqwest::Client,
    transport: reqwest13::Client,
}

impl McpHttp {
    pub fn new(policy: NetworkPolicy) -> Result<Self, CoreError> {
        Ok(Self {
            policy,
            client: guard::build_client(policy)?,
            transport: gateway::build_transport(policy, CONNECT_TIMEOUT)?,
        })
    }

    pub fn policy(&self) -> NetworkPolicy {
        self.policy
    }
}

pub(crate) fn refusal(what: &str, failure: &ConnectorFailure) -> CoreError {
    let reason = match failure {
        ConnectorFailure::Blocked(_) => "adresse refusée par la politique réseau",
        ConnectorFailure::Timeout => "délai dépassé",
        ConnectorFailure::Unreachable(_) => "injoignable",
        ConnectorFailure::Unauthorized => "accès refusé",
        _ => "réponse inattendue",
    };
    CoreError::Invalid(format!("{what} : {reason}"))
}
