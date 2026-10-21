use std::collections::HashMap;

use capitainerie_common::IntegrationInstanceId;
use capitainerie_core::domain::integration::{
    ConnectorFailure, InstanceConnection, ProbeReport, action::ActionRequest,
};
use reqwest::Client;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::guard::{clean_version, remote_failure, scrub, send};
use crate::odoo::{self, OdooCall, RpcArgs, RpcKwargs, Transport};

const DEPRECATION_WARNING: &str =
    "JSON-RPC is deprecated since Odoo 19 and removed in Odoo 22: prefer the Odoo 19+ type";
const FIRST_DEPRECATED_MAJOR: u32 = 19;

enum Fault {
    AccessDenied,
    Other(String),
}

enum Attempt {
    Denied,
    Failed(ConnectorFailure),
}

impl Attempt {
    fn into_failure(self) -> ConnectorFailure {
        match self {
            Self::Denied => ConnectorFailure::Unauthorized,
            Self::Failed(failure) => failure,
        }
    }
}

impl From<ConnectorFailure> for Attempt {
    fn from(failure: ConnectorFailure) -> Self {
        Self::Failed(failure)
    }
}

#[derive(Default)]
pub(crate) struct Sessions {
    uids: Mutex<HashMap<IntegrationInstanceId, i64>>,
}

impl Sessions {
    async fn uid(&self, instance: IntegrationInstanceId) -> Option<i64> {
        self.uids.lock().await.get(&instance).copied()
    }

    async fn remember(&self, instance: IntegrationInstanceId, uid: i64) {
        self.uids.lock().await.insert(instance, uid);
    }

    async fn forget(&self, instance: IntegrationInstanceId) {
        self.uids.lock().await.remove(&instance);
    }
}

#[derive(Serialize)]
struct Envelope<P> {
    jsonrpc: &'static str,
    method: &'static str,
    params: P,
    id: u8,
}

#[derive(Serialize)]
struct ObjectParams<'a> {
    service: &'static str,
    method: &'static str,
    args: (
        &'a str,
        i64,
        &'a str,
        &'a str,
        &'a str,
        RpcArgs<'a, 'a>,
        RpcKwargs<'a, 'a>,
    ),
}

pub(crate) struct OdooJsonRpc<'a> {
    client: &'a Client,
    connection: &'a InstanceConnection,
    sessions: &'a Sessions,
}

impl<'a> OdooJsonRpc<'a> {
    pub(crate) fn new(
        client: &'a Client,
        connection: &'a InstanceConnection,
        sessions: &'a Sessions,
    ) -> Self {
        Self {
            client,
            connection,
            sessions,
        }
    }

    fn identity(&self) -> Result<(&str, &str), ConnectorFailure> {
        match (&self.connection.database, &self.connection.login) {
            (Some(database), Some(login)) => Ok((database, login)),
            _ => Err(ConnectorFailure::Protocol(
                "database and login are required for JSON-RPC".into(),
            )),
        }
    }

    pub(crate) async fn probe(&self) -> Result<ProbeReport, ConnectorFailure> {
        let (database, login) = self.identity()?;
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
        self.authenticate(database, login).await?;
        Ok(ProbeReport {
            warning: deprecation_warning(&server_version),
            server_version,
        })
    }

    pub(crate) async fn perform(&self, request: &ActionRequest) -> Result<Value, ConnectorFailure> {
        odoo::perform(self, request).await
    }

    async fn authenticate(&self, database: &str, login: &str) -> Result<i64, ConnectorFailure> {
        let secret = self.connection.secret.expose();
        let reply = self
            .call("authenticate", json!([database, login, secret, {}]))
            .await?;
        if let Some(fault) = fault_of(&reply, secret) {
            return Err(fault_failure(fault));
        }
        match reply.get("result") {
            Some(Value::Bool(false)) => Err(ConnectorFailure::Unauthorized),
            Some(Value::Number(uid)) => match uid.as_i64() {
                Some(0) => Err(ConnectorFailure::Unauthorized),
                Some(uid) if uid > 0 => Ok(uid),
                _ => Err(unexpected_login()),
            },
            _ => Err(unexpected_login()),
        }
    }

    async fn login(&self, database: &str, login: &str) -> Result<i64, ConnectorFailure> {
        let uid = self.authenticate(database, login).await?;
        self.sessions
            .remember(self.connection.instance_id, uid)
            .await;
        Ok(uid)
    }

    async fn object(
        &self,
        database: &str,
        uid: i64,
        call: &OdooCall<'_>,
    ) -> Result<Value, Attempt> {
        let secret = self.connection.secret.expose();
        let body = Envelope {
            jsonrpc: "2.0",
            method: "call",
            params: ObjectParams {
                service: "object",
                method: "execute_kw",
                args: (
                    database,
                    uid,
                    secret,
                    call.model,
                    call.method,
                    RpcArgs(call),
                    RpcKwargs(call),
                ),
            },
            id: 1,
        };
        let mut reply = self.post(&body).await?;
        match fault_of(&reply, secret) {
            Some(Fault::AccessDenied) => return Err(Attempt::Denied),
            Some(Fault::Other(message)) => {
                return Err(Attempt::Failed(ConnectorFailure::Remote {
                    status: 200,
                    message,
                }));
            }
            None => {}
        }
        reply.get_mut("result").map(Value::take).ok_or_else(|| {
            Attempt::Failed(ConnectorFailure::Protocol(
                "the server did not report a result".into(),
            ))
        })
    }

    async fn call(&self, method: &str, args: Value) -> Result<Value, ConnectorFailure> {
        self.post(&json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": {"service": "common", "method": method, "args": args},
            "id": 1,
        }))
        .await
    }

    async fn post<B: Serialize>(&self, body: &B) -> Result<Value, ConnectorFailure> {
        let answer = send(
            self.client
                .post(self.connection.base_url.join("jsonrpc"))
                .json(body),
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

impl Transport for OdooJsonRpc<'_> {
    async fn execute(&self, call: &OdooCall<'_>) -> Result<Value, ConnectorFailure> {
        let (database, login) = self.identity()?;
        let instance = self.connection.instance_id;
        let (uid, cached) = match self.sessions.uid(instance).await {
            Some(uid) => (uid, true),
            None => (self.login(database, login).await?, false),
        };
        let first = self.object(database, uid, call).await;
        if !matches!(first, Err(Attempt::Denied)) {
            return first.map_err(Attempt::into_failure);
        }
        self.sessions.forget(instance).await;
        if !cached {
            return Err(ConnectorFailure::Unauthorized);
        }
        let uid = self.login(database, login).await?;
        let second = self.object(database, uid, call).await;
        if matches!(second, Err(Attempt::Denied)) {
            self.sessions.forget(instance).await;
        }
        second.map_err(Attempt::into_failure)
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
