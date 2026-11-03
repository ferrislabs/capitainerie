use capitainerie_core::domain::integration::{ConnectorFailure, InstanceConnection, ProbeReport};
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

use crate::guard::{clean_version, remote_failure, secret_header, send};

const UNKNOWN_VERSION: &str = "19+";

pub(crate) struct OdooJson2<'a> {
    client: &'a Client,
    connection: &'a InstanceConnection,
}

impl<'a> OdooJson2<'a> {
    pub(crate) fn new(client: &'a Client, connection: &'a InstanceConnection) -> Self {
        Self { client, connection }
    }

    pub(crate) async fn probe(&self) -> Result<ProbeReport, ConnectorFailure> {
        let version = self.server_version().await;
        let secret = self.connection.secret.expose();
        let mut request = self
            .client
            .post(
                self.connection
                    .base_url
                    .join("json/2/res.users/context_get"),
            )
            .header(AUTHORIZATION, secret_header("bearer ", secret)?)
            .header(CONTENT_TYPE, "application/json")
            .body("{}");
        if let Some(database) = &self.connection.database {
            request = request.header("X-Odoo-Database", database);
        }
        let answer = send(request).await?;
        match answer.status.as_u16() {
            200 => Ok(ProbeReport {
                server_version: version.unwrap_or_else(|| UNKNOWN_VERSION.to_owned()),
                warning: None,
            }),
            401 | 403 => Err(ConnectorFailure::Unauthorized),
            404 => Err(ConnectorFailure::VersionMismatch(
                "this server does not serve the JSON-2 API (Odoo 19 or later)".into(),
            )),
            _ => Err(remote_failure(&answer, secret)),
        }
    }

    async fn server_version(&self) -> Option<String> {
        let answer = send(
            self.client
                .get(self.connection.base_url.join("web/version")),
        )
        .await
        .ok()?;
        if answer.status.as_u16() != 200 {
            return None;
        }
        let body = answer.json()?;
        body.get("version")
            .and_then(Value::as_str)
            .filter(|version| !version.is_empty())
            .map(clean_version)
    }
}
