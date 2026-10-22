use capitainerie_core::domain::integration::{
    ConnectorFailure, InstanceConnection, IntegrationKind, ProbeReport,
};
use serde_json::{Value, json};

use crate::support::{Fake, Reply, SECRET, connection, ok, probe, serve};

fn rpc(result: Value) -> Reply {
    Reply::json(200, &json!({"jsonrpc": "2.0", "id": 1, "result": result}))
}

fn rpc_error(name: &str, message: &str) -> Reply {
    Reply::json(
        200,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": 200,
                "message": "Odoo Server Error",
                "data": {"name": name, "message": message}
            }
        }),
    )
}

async fn odoo(version: &'static str, login: Value) -> Fake {
    serve(move |recorded| {
        if recorded.path != "/jsonrpc" {
            return Reply::status(404);
        }
        match recorded.json()["params"]["method"].as_str() {
            Some("version") => rpc(json!({"server_version": version, "protocol_version": 1})),
            Some("authenticate") => rpc(login.clone()),
            _ => Reply::status(400),
        }
    })
    .await
}

fn jsonrpc(base: &str) -> InstanceConnection {
    connection(
        IntegrationKind::OdooJsonRpc,
        base,
        Some("prod"),
        Some("admin@acme.test"),
    )
}

#[tokio::test]
async fn a_positive_uid_is_a_successful_login() {
    let fake = odoo("17.0", json!(7)).await;
    assert_eq!(probe(&jsonrpc(&fake.base)).await, ok("17.0"));
}

#[tokio::test]
async fn the_calls_follow_the_json_rpc_envelope() {
    let fake = odoo("17.0", json!(7)).await;
    probe(&jsonrpc(&fake.base)).await.unwrap();
    let calls = fake.requests();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].json(),
        json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": {"service": "common", "method": "version", "args": []},
            "id": 1
        })
    );
    assert_eq!(
        calls[1].json(),
        json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": {
                "service": "common",
                "method": "authenticate",
                "args": ["prod", "admin@acme.test", SECRET, {}]
            },
            "id": 1
        })
    );
}

#[tokio::test]
async fn a_false_or_zero_uid_is_unauthorized() {
    for login in [json!(false), json!(0)] {
        let fake = odoo("17.0", login).await;
        assert_eq!(
            probe(&jsonrpc(&fake.base)).await,
            Err(ConnectorFailure::Unauthorized)
        );
    }
}

#[tokio::test]
async fn an_access_denied_error_is_unauthorized() {
    let fake = serve(
        |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("version") => rpc(json!({"server_version": "16.0"})),
            _ => rpc_error("odoo.exceptions.AccessDenied", "Access Denied"),
        },
    )
    .await;
    assert_eq!(
        probe(&jsonrpc(&fake.base)).await,
        Err(ConnectorFailure::Unauthorized)
    );
}

#[tokio::test]
async fn another_rpc_error_is_a_protocol_failure_with_its_message() {
    let fake = serve(
        |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("version") => rpc(json!({"server_version": "16.0"})),
            _ => rpc_error(
                "psycopg2.OperationalError",
                "database \"prod\" does not exist",
            ),
        },
    )
    .await;
    assert_eq!(
        probe(&jsonrpc(&fake.base)).await,
        Err(ConnectorFailure::Protocol(
            "database \"prod\" does not exist".into()
        ))
    );
}

#[tokio::test]
async fn a_rpc_error_message_never_carries_the_secret() {
    let fake = serve(
        |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("version") => rpc(json!({"server_version": "16.0"})),
            _ => rpc_error("builtins.ValueError", &format!("bad key {SECRET}")),
        },
    )
    .await;
    assert_eq!(
        probe(&jsonrpc(&fake.base)).await,
        Err(ConnectorFailure::Protocol("bad key ***".into()))
    );
}

#[tokio::test]
async fn a_missing_jsonrpc_route_is_a_version_mismatch() {
    let fake = serve(|_| Reply::status(404)).await;
    assert!(matches!(
        probe(&jsonrpc(&fake.base)).await,
        Err(ConnectorFailure::VersionMismatch(message)) if message.contains("removed in Odoo 22")
    ));
    assert_eq!(fake.hits(), 1);
}

#[tokio::test]
async fn odoo_19_and_later_carry_a_deprecation_warning() {
    for (version, warned) in [
        ("19.0", true),
        ("saas~19.1", true),
        ("17.0", false),
        ("saas~18.3", false),
    ] {
        let fake = odoo(version, json!(2)).await;
        let report = probe(&jsonrpc(&fake.base)).await.unwrap();
        assert_eq!(report.server_version, version);
        assert_eq!(
            report.warning.as_deref(),
            warned.then_some(
                "JSON-RPC is deprecated since Odoo 19 and removed in Odoo 22: prefer the Odoo 19+ type"
            ),
            "version {version}"
        );
    }
}

#[tokio::test]
async fn a_database_and_a_login_are_required_before_any_request() {
    let fake = odoo("17.0", json!(7)).await;
    for (database, login) in [(None, Some("admin")), (Some("prod"), None), (None, None)] {
        let connection = connection(IntegrationKind::OdooJsonRpc, &fake.base, database, login);
        assert_eq!(
            probe(&connection).await,
            Err(ConnectorFailure::Protocol(
                "database and login are required for JSON-RPC".into()
            ))
        );
    }
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn a_server_that_is_not_odoo_is_a_protocol_failure() {
    let fake = serve(|_| Reply::json(200, &json!({"hello": "world"}))).await;
    assert!(matches!(
        probe(&jsonrpc(&fake.base)).await,
        Err(ConnectorFailure::Protocol(_))
    ));
    let fake = serve(|_| Reply::bytes(200, 12)).await;
    assert!(matches!(
        probe(&jsonrpc(&fake.base)).await,
        Err(ConnectorFailure::Protocol(_))
    ));
}

#[tokio::test]
async fn the_report_has_the_version_and_no_warning_for_an_old_server() {
    let fake = odoo("16.0", json!(3)).await;
    assert_eq!(
        probe(&jsonrpc(&fake.base)).await,
        Ok(ProbeReport {
            server_version: "16.0".into(),
            warning: None
        })
    );
}
