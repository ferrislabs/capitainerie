use capitainerie_core::domain::integration::{
    ConnectorFailure, InstanceConnection, IntegrationKind,
};
use serde_json::json;

use crate::support::{Fake, Recorded, Reply, SECRET, connection, ok, probe, serve};

async fn n8n(workflows: u16, discover: u16) -> Fake {
    serve(move |recorded| match recorded.path.as_str() {
        "/api/v1/workflows?limit=1" if workflows == 200 => {
            Reply::json(200, &json!({"data": [], "nextCursor": null}))
        }
        "/api/v1/workflows?limit=1" => Reply::status(workflows),
        "/api/v1/discover" if discover == 200 => Reply::json(200, &json!({"paths": {}})),
        "/api/v1/discover" => Reply::status(discover),
        _ => Reply::status(500),
    })
    .await
}

fn instance(base: &str) -> InstanceConnection {
    connection(IntegrationKind::N8n, base, None, None)
}

#[tokio::test]
async fn a_missing_discover_endpoint_means_version_1() {
    let fake = n8n(200, 404).await;
    assert_eq!(probe(&instance(&fake.base)).await, ok("1"));
}

#[tokio::test]
async fn any_other_discover_answer_means_version_2() {
    for status in [200, 401, 500] {
        let fake = n8n(200, status).await;
        assert_eq!(
            probe(&instance(&fake.base)).await,
            ok("2"),
            "discover {status}"
        );
    }
}

#[tokio::test]
async fn refused_credentials_are_unauthorized_without_asking_discover() {
    for status in [401, 403] {
        let fake = n8n(status, 200).await;
        assert_eq!(
            probe(&instance(&fake.base)).await,
            Err(ConnectorFailure::Unauthorized)
        );
        assert_eq!(fake.hits(), 1);
    }
}

#[tokio::test]
async fn a_missing_public_api_is_a_protocol_failure() {
    let fake = n8n(404, 200).await;
    assert!(matches!(
        probe(&instance(&fake.base)).await,
        Err(ConnectorFailure::Protocol(message)) if message.contains("public API")
    ));
}

#[tokio::test]
async fn another_status_is_a_remote_failure() {
    let fake = n8n(500, 200).await;
    assert_eq!(
        probe(&instance(&fake.base)).await,
        Err(ConnectorFailure::Remote {
            status: 500,
            message: "Internal Server Error".into()
        })
    );
}

#[tokio::test]
async fn the_api_key_is_sent_in_its_header_on_both_calls() {
    let fake = n8n(200, 200).await;
    probe(&instance(&fake.base)).await.unwrap();
    let calls: Vec<Recorded> = fake.requests();
    assert_eq!(calls.len(), 2);
    for call in calls {
        assert_eq!(call.method, "GET");
        assert_eq!(call.header("x-n8n-api-key"), Some(SECRET));
        assert_eq!(call.header("accept"), Some("application/json"));
        assert_eq!(call.header("authorization"), None);
    }
}

#[tokio::test]
async fn the_base_url_sub_path_is_kept() {
    let fake = serve(|recorded| match recorded.path.as_str() {
        "/automation/api/v1/workflows?limit=1" => Reply::json(200, &json!({"data": []})),
        "/automation/api/v1/discover" => Reply::status(404),
        _ => Reply::status(500),
    })
    .await;
    let base = format!("{}/automation", fake.base);
    assert_eq!(probe(&instance(&base)).await, ok("1"));
}
