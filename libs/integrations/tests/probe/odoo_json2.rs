use capitainerie_core::domain::integration::{
    ConnectorFailure, InstanceConnection, IntegrationKind,
};
use serde_json::json;

use crate::support::{Fake, Recorded, Reply, SECRET, connection, ok, probe, serve};

async fn odoo(version: u16, context: u16) -> Fake {
    serve(
        move |recorded| match (recorded.method.as_str(), recorded.path.as_str()) {
            ("GET", "/web/version") if version == 200 => Reply::json(
                200,
                &json!({"version": "19.0", "version_info": [19, 0, 0, "final", 0, ""]}),
            ),
            ("GET", "/web/version") => Reply::status(version),
            ("POST", "/json/2/res.users/context_get") if context == 200 => {
                Reply::json(200, &json!({"lang": "en_US", "tz": "Europe/Paris"}))
            }
            ("POST", "/json/2/res.users/context_get") => Reply::status(context),
            _ => Reply::status(500),
        },
    )
    .await
}

fn json2(base: &str, database: Option<&str>) -> InstanceConnection {
    connection(IntegrationKind::OdooJson2, base, database, None)
}

#[tokio::test]
async fn the_version_comes_from_web_version_when_the_context_call_succeeds() {
    let fake = odoo(200, 200).await;
    assert_eq!(probe(&json2(&fake.base, Some("prod"))).await, ok("19.0"));
}

#[tokio::test]
async fn the_version_defaults_to_19_plus_when_web_version_is_missing() {
    let fake = odoo(404, 200).await;
    assert_eq!(probe(&json2(&fake.base, None)).await, ok("19+"));
    let fake = odoo(500, 200).await;
    assert_eq!(probe(&json2(&fake.base, None)).await, ok("19+"));
}

#[tokio::test]
async fn refused_credentials_are_unauthorized() {
    for status in [401, 403] {
        let fake = odoo(200, status).await;
        assert_eq!(
            probe(&json2(&fake.base, Some("prod"))).await,
            Err(ConnectorFailure::Unauthorized)
        );
    }
}

#[tokio::test]
async fn a_server_without_the_json2_route_is_a_version_mismatch() {
    let fake = odoo(200, 404).await;
    assert!(matches!(
        probe(&json2(&fake.base, None)).await,
        Err(ConnectorFailure::VersionMismatch(message)) if message.contains("JSON-2")
    ));
}

#[tokio::test]
async fn the_database_header_is_sent_only_when_a_database_is_set() {
    let fake = odoo(200, 200).await;
    probe(&json2(&fake.base, Some("prod"))).await.unwrap();
    probe(&json2(&fake.base, None)).await.unwrap();
    let calls: Vec<Recorded> = fake
        .requests()
        .into_iter()
        .filter(|recorded| recorded.method == "POST")
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].header("x-odoo-database"), Some("prod"));
    assert_eq!(calls[1].header("x-odoo-database"), None);
}

#[tokio::test]
async fn the_secret_travels_as_a_bearer_token_with_an_empty_json_body() {
    let fake = odoo(200, 200).await;
    probe(&json2(&fake.base, None)).await.unwrap();
    let call = fake.request("POST", "/json/2/res.users/context_get");
    assert_eq!(
        call.header("authorization"),
        Some(format!("bearer {SECRET}").as_str())
    );
    assert_eq!(call.header("content-type"), Some("application/json"));
    assert_eq!(call.body, "{}");
}

#[tokio::test]
async fn the_base_url_sub_path_is_kept() {
    let fake = serve(|recorded| match recorded.path.as_str() {
        "/erp/web/version" => Reply::json(200, &json!({"version": "20.0"})),
        "/erp/json/2/res.users/context_get" => Reply::json(200, &json!({})),
        _ => Reply::status(404),
    })
    .await;
    let base = format!("{}/erp/", fake.base);
    assert_eq!(probe(&json2(&base, None)).await, ok("20.0"));
}

#[tokio::test]
async fn another_status_becomes_a_remote_failure_with_odoos_message() {
    let fake = serve(|recorded| match recorded.path.as_str() {
        "/json/2/res.users/context_get" => Reply::json(
            500,
            &json!({"name": "odoo.exceptions.UserError", "message": "boom"}),
        ),
        _ => Reply::status(404),
    })
    .await;
    assert_eq!(
        probe(&json2(&fake.base, None)).await,
        Err(ConnectorFailure::Remote {
            status: 500,
            message: "boom".into()
        })
    );
}

#[tokio::test]
async fn a_remote_message_is_truncated_and_never_carries_the_secret() {
    let long = format!("{SECRET} {}", "x".repeat(500));
    let fake = serve(move |recorded| match recorded.path.as_str() {
        "/json/2/res.users/context_get" => Reply::json(502, &json!({"message": long})),
        _ => Reply::status(404),
    })
    .await;
    let Err(ConnectorFailure::Remote { status, message }) = probe(&json2(&fake.base, None)).await
    else {
        panic!("expected a remote failure");
    };
    assert_eq!(status, 502);
    assert_eq!(message.chars().count(), 300);
    assert!(message.starts_with("*** "));
}

#[tokio::test]
async fn a_remote_failure_without_a_json_body_falls_back_to_the_status_reason() {
    let fake = odoo(200, 503).await;
    assert_eq!(
        probe(&json2(&fake.base, None)).await,
        Err(ConnectorFailure::Remote {
            status: 503,
            message: "Service Unavailable".into()
        })
    );
}
