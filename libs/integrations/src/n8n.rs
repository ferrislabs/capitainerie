use capitainerie_core::domain::integration::{ConnectorFailure, InstanceConnection, ProbeReport};
use reqwest::Client;
use reqwest::header::{ACCEPT, HeaderValue};

use crate::guard::{remote_failure, secret_header, send};

const API_KEY_HEADER: &str = "X-N8N-API-KEY";
const DISCOVERY_VERSION: &str = "2";
const LEGACY_VERSION: &str = "1";

pub(crate) struct N8n<'a> {
    client: &'a Client,
    connection: &'a InstanceConnection,
}

impl<'a> N8n<'a> {
    pub(crate) fn new(client: &'a Client, connection: &'a InstanceConnection) -> Self {
        Self { client, connection }
    }

    pub(crate) async fn probe(&self) -> Result<ProbeReport, ConnectorFailure> {
        let secret = self.connection.secret.expose();
        let key = secret_header("", secret)?;
        let workflows = send(
            self.client
                .get(self.connection.base_url.join("api/v1/workflows?limit=1"))
                .header(API_KEY_HEADER, key.clone())
                .header(ACCEPT, HeaderValue::from_static("application/json")),
        )
        .await?;
        match workflows.status.as_u16() {
            200 => {}
            401 | 403 => return Err(ConnectorFailure::Unauthorized),
            404 => {
                return Err(ConnectorFailure::Protocol(
                    "no n8n public API at this URL (is the public API enabled?)".into(),
                ));
            }
            _ => return Err(remote_failure(&workflows, secret)),
        }
        let discovery = send(
            self.client
                .get(self.connection.base_url.join("api/v1/discover"))
                .header(API_KEY_HEADER, key)
                .header(ACCEPT, HeaderValue::from_static("application/json")),
        )
        .await?;
        let server_version = if discovery.status.as_u16() == 404 {
            LEGACY_VERSION
        } else {
            DISCOVERY_VERSION
        };
        Ok(ProbeReport {
            server_version: server_version.to_owned(),
            warning: None,
        })
    }
}
