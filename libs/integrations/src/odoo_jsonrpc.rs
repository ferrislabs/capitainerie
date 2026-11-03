use capitainerie_core::domain::integration::{ConnectorFailure, InstanceConnection, ProbeReport};
use reqwest::Client;
use serde_json::{Value, json};

use crate::guard::{clean_version, remote_failure, scrub, send};

const DEPRECATION_WARNING: &str =
    "JSON-RPC is deprecated since Odoo 19 and removed in Odoo 22: prefer the Odoo 19+ type";
const FIRST_DEPRECATED_MAJOR: u32 = 19;

enum Fault {
    AccessDenied,
    Other(String),
}

pub(crate) struct OdooJsonRpc<'a> {
    client: &'a Client,
    connection: &'a InstanceConnection,
}

impl<'a> OdooJsonRpc<'a> {
    pub(crate) fn new(client: &'a Client, connection: &'a InstanceConnection) -> Self {
        Self { client, connection }
    }

    pub(crate) async fn probe(&self) -> Result<ProbeReport, ConnectorFailure> {
        let (Some(database), Some(login)) = (&self.connection.database, &self.connection.login)
        else {
            return Err(ConnectorFailure::Protocol(
                "database and login are required for JSON-RPC".into(),
            ));
        };
        let secret = self.connection.secret.expose();
        let version_reply = self.call("version", json!([])).await?;
        if let Some(fault) = fault_of(&version_reply, secret) {
            return Err(fault_failure(fault));
        }
        let server_version = version_reply
            .pointer("/result/server_version")
            .and_then(Value::as_str)
            .map(clean_version)
            .ok_or_else(|| {
                ConnectorFailure::Protocol("the server did not report its Odoo version".into())
            })?;
        let login_reply = self
            .call("authenticate", json!([database, login, secret, {}]))
            .await?;
        if let Some(fault) = fault_of(&login_reply, secret) {
            return Err(fault_failure(fault));
        }
        match login_reply.get("result") {
            Some(Value::Bool(false)) => Err(ConnectorFailure::Unauthorized),
            Some(Value::Number(uid)) => match uid.as_u64() {
                Some(0) => Err(ConnectorFailure::Unauthorized),
                Some(_) => Ok(ProbeReport {
                    warning: deprecation_warning(&server_version),
                    server_version,
                }),
                None => Err(unexpected_login()),
            },
            _ => Err(unexpected_login()),
        }
    }

    async fn call(&self, method: &str, args: Value) -> Result<Value, ConnectorFailure> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": {"service": "common", "method": method, "args": args},
            "id": 1,
        });
        let answer = send(
            self.client
                .post(self.connection.base_url.join("jsonrpc"))
                .json(&body),
        )
        .await?;
        match answer.status.as_u16() {
            200 => answer.json().ok_or_else(|| {
                ConnectorFailure::Protocol("the server did not answer with JSON".into())
            }),
            401 | 403 => Err(ConnectorFailure::Unauthorized),
            404 => Err(ConnectorFailure::VersionMismatch(
                "this server does not serve JSON-RPC (removed in Odoo 22)".into(),
            )),
            _ => Err(remote_failure(&answer, self.connection.secret.expose())),
        }
    }
}

fn unexpected_login() -> ConnectorFailure {
    ConnectorFailure::Protocol("the server answered something unexpected to the login".into())
}

fn fault_failure(fault: Fault) -> ConnectorFailure {
    match fault {
        Fault::AccessDenied => ConnectorFailure::Unauthorized,
        Fault::Other(message) => ConnectorFailure::Protocol(message),
    }
}

fn fault_of(reply: &Value, secret: &str) -> Option<Fault> {
    let error = reply.get("error").filter(|error| !error.is_null())?;
    let data = error.get("data");
    let denied = data
        .and_then(|data| data.get("name"))
        .and_then(Value::as_str)
        .is_some_and(|name| name.contains("AccessDenied"));
    if denied {
        return Some(Fault::AccessDenied);
    }
    let message = data
        .and_then(|data| data.get("message"))
        .and_then(Value::as_str)
        .or_else(|| error.get("message").and_then(Value::as_str))
        .unwrap_or("the server reported an error");
    Some(Fault::Other(scrub(message, secret)))
}

fn major_version(version: &str) -> Option<u32> {
    let digits: String = version
        .strip_prefix("saas~")
        .unwrap_or(version)
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn deprecation_warning(version: &str) -> Option<String> {
    major_version(version)
        .filter(|major| *major >= FIRST_DEPRECATED_MAJOR)
        .map(|_| DEPRECATION_WARNING.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_major_version_ignores_the_saas_prefix_and_the_minor() {
        assert_eq!(major_version("17.0"), Some(17));
        assert_eq!(major_version("saas~18.3"), Some(18));
        assert_eq!(major_version("saas~19.1"), Some(19));
        assert_eq!(major_version("19.0+e"), Some(19));
        assert_eq!(major_version("master"), None);
    }

    #[test]
    fn only_odoo_19_and_later_get_the_deprecation_warning() {
        assert!(deprecation_warning("19.0").is_some());
        assert!(deprecation_warning("saas~19.1").is_some());
        assert!(deprecation_warning("21.0").is_some());
        assert!(deprecation_warning("18.0").is_none());
        assert!(deprecation_warning("saas~18.3").is_none());
        assert!(deprecation_warning("unknown").is_none());
    }

    #[test]
    fn an_access_denied_fault_is_told_apart_from_other_faults() {
        let denied = json!({"error": {"message": "Odoo Server Error", "data": {"name": "odoo.exceptions.AccessDenied", "message": "Access Denied"}}});
        assert!(matches!(fault_of(&denied, "k"), Some(Fault::AccessDenied)));
        let other = json!({"error": {"message": "Odoo Server Error", "data": {"name": "psycopg2.OperationalError", "message": "database \"x\" does not exist"}}});
        assert!(matches!(
            fault_of(&other, "k"),
            Some(Fault::Other(message)) if message.contains("does not exist")
        ));
        assert!(fault_of(&json!({"result": 7}), "k").is_none());
        assert!(fault_of(&json!({"error": null, "result": 7}), "k").is_none());
    }
}
