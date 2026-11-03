mod guard;
mod n8n;
mod odoo_json2;
mod odoo_jsonrpc;

use capitainerie_common::CoreError;
use capitainerie_core::domain::integration::{
    ConnectorFailure, InstanceConnection, IntegrationKind, ProbeReport, ports::IntegrationConnector,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetworkPolicy {
    pub allow_insecure_http: bool,
    pub allow_private_networks: bool,
}

#[derive(Clone)]
pub struct HttpConnector {
    policy: NetworkPolicy,
    client: reqwest::Client,
}

impl HttpConnector {
    pub fn new(policy: NetworkPolicy) -> Result<Self, CoreError> {
        Ok(Self {
            policy,
            client: guard::build_client(policy)?,
        })
    }

    pub fn policy(&self) -> NetworkPolicy {
        self.policy
    }
}

impl IntegrationConnector for HttpConnector {
    async fn probe(
        &self,
        connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        guard::check_target(self.policy, &connection.base_url).await?;
        match connection.kind {
            IntegrationKind::OdooJson2 => {
                odoo_json2::OdooJson2::new(&self.client, connection)
                    .probe()
                    .await
            }
            IntegrationKind::OdooJsonRpc => {
                odoo_jsonrpc::OdooJsonRpc::new(&self.client, connection)
                    .probe()
                    .await
            }
            IntegrationKind::N8n => n8n::N8n::new(&self.client, connection).probe().await,
        }
    }
}
