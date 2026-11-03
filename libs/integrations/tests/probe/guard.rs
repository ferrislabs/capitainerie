use capitainerie_core::domain::integration::{ConnectorFailure, IntegrationKind};
use capitainerie_integrations::{HttpConnector, NetworkPolicy};
use serde_json::json;

use crate::support::{OPEN, Reply, connection, probe, probe_with, serve};

const STRICT: NetworkPolicy = NetworkPolicy {
    allow_insecure_http: false,
    allow_private_networks: false,
};

const PRIVATE_ONLY: NetworkPolicy = NetworkPolicy {
    allow_insecure_http: false,
    allow_private_networks: true,
};

const HTTP_ONLY: NetworkPolicy = NetworkPolicy {
    allow_insecure_http: true,
    allow_private_networks: false,
};

fn is_blocked(outcome: &Result<impl Sized, ConnectorFailure>) -> bool {
    matches!(outcome, Err(ConnectorFailure::Blocked(_)))
}

#[test]
fn the_connector_keeps_its_policy_and_is_cloneable() {
    let connector = HttpConnector::new(PRIVATE_ONLY).unwrap();
    assert_eq!(connector.clone().policy(), PRIVATE_ONLY);
    assert_eq!(NetworkPolicy::default(), STRICT);
}

#[tokio::test]
async fn plain_http_to_a_loopback_fake_is_blocked_before_any_connection() {
    let fake = serve(|_| Reply::json(200, &json!({}))).await;
    for kind in IntegrationKind::ALL {
        let connection = connection(kind, &fake.base, Some("prod"), Some("admin"));
        let outcome = probe_with(NetworkPolicy::default(), &connection).await;
        assert!(is_blocked(&outcome), "{kind:?}: {outcome:?}");
    }
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn allowing_plain_http_does_not_allow_a_loopback_address() {
    let fake = serve(|_| Reply::json(200, &json!({}))).await;
    let connection = connection(IntegrationKind::OdooJson2, &fake.base, None, None);
    assert!(is_blocked(&probe_with(HTTP_ONLY, &connection).await));
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn plain_http_is_blocked_unless_it_is_allowed_even_on_private_networks() {
    let fake = serve(|_| Reply::json(200, &json!({}))).await;
    let connection = connection(IntegrationKind::OdooJson2, &fake.base, None, None);
    let outcome = probe_with(PRIVATE_ONLY, &connection).await;
    assert!(
        matches!(&outcome, Err(ConnectorFailure::Blocked(message)) if message.contains("plain http")),
        "{outcome:?}"
    );
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn private_special_and_mapped_address_literals_are_blocked() {
    for base in [
        "https://10.0.0.5",
        "https://192.168.1.10:8069",
        "https://172.16.0.1",
        "https://169.254.169.254",
        "https://100.64.0.1",
        "https://127.0.0.1:1",
        "https://0.0.0.0",
        "https://[::1]",
        "https://[::]",
        "https://[fd00::1]",
        "https://[fe80::1]",
        "https://[ff02::1]",
        "https://[::ffff:127.0.0.1]",
        "https://[::ffff:10.0.0.5]:8443",
        "https://0x7f.1",
        "https://2130706433",
    ] {
        for kind in IntegrationKind::ALL {
            let connection = connection(kind, base, Some("prod"), Some("admin"));
            let outcome = probe_with(STRICT, &connection).await;
            assert!(is_blocked(&outcome), "{base} {kind:?}: {outcome:?}");
        }
    }
}

#[tokio::test]
async fn a_blocked_literal_names_the_address() {
    let connection = connection(IntegrationKind::N8n, "https://10.0.0.5", None, None);
    assert_eq!(
        probe_with(STRICT, &connection).await,
        Err(ConnectorFailure::Blocked("10.0.0.5".into()))
    );
}

#[tokio::test]
async fn a_hostname_resolving_to_loopback_is_blocked_before_any_connection() {
    let fake = serve(|_| Reply::json(200, &json!({}))).await;
    let base = format!("http://localhost:{}", fake.port);
    let connection = connection(IntegrationKind::OdooJson2, &base, None, None);
    assert_eq!(
        probe_with(HTTP_ONLY, &connection).await,
        Err(ConnectorFailure::Blocked("localhost".into()))
    );
    let base = format!("https://localhost:{}", fake.port);
    let connection = self::connection(IntegrationKind::OdooJson2, &base, None, None);
    assert_eq!(
        probe_with(STRICT, &connection).await,
        Err(ConnectorFailure::Blocked("localhost".into()))
    );
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn an_unresolvable_name_is_unreachable() {
    let connection = connection(
        IntegrationKind::OdooJson2,
        "https://no-such-host.invalid",
        None,
        None,
    );
    assert!(matches!(
        probe_with(STRICT, &connection).await,
        Err(ConnectorFailure::Unreachable(_))
    ));
}

#[tokio::test]
async fn a_closed_port_is_unreachable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let connection = connection(IntegrationKind::OdooJson2, &base, None, None);
    assert!(matches!(
        probe(&connection).await,
        Err(ConnectorFailure::Unreachable(_))
    ));
}

#[tokio::test]
async fn a_redirect_is_not_followed() {
    let target = serve(|_| Reply::json(200, &json!({}))).await;
    let location = format!("{}/json/2/res.users/context_get", target.base);
    let fake = serve(move |_| Reply::redirect(&location)).await;
    for kind in IntegrationKind::ALL {
        let connection = connection(kind, &fake.base, Some("prod"), Some("admin"));
        assert_eq!(
            probe(&connection).await,
            Err(ConnectorFailure::Protocol(
                "redirects are not followed".into()
            )),
            "{kind:?}"
        );
    }
    assert_eq!(target.hits(), 0);
}

#[tokio::test]
async fn a_body_over_10_mib_is_refused() {
    let fake = serve(|_| Reply::bytes(200, 10 * 1024 * 1024 + 1)).await;
    for kind in IntegrationKind::ALL {
        let connection = connection(kind, &fake.base, Some("prod"), Some("admin"));
        assert_eq!(
            probe(&connection).await,
            Err(ConnectorFailure::Protocol(
                "the answer exceeds 10 MiB".into()
            )),
            "{kind:?}"
        );
    }
}

#[tokio::test]
async fn a_streamed_body_over_10_mib_is_cut_off_while_reading() {
    let fake = serve(|_| Reply::stream(11, 1024 * 1024)).await;
    let connection = connection(
        IntegrationKind::OdooJsonRpc,
        &fake.base,
        Some("prod"),
        Some("admin"),
    );
    assert_eq!(
        probe(&connection).await,
        Err(ConnectorFailure::Protocol(
            "the answer exceeds 10 MiB".into()
        ))
    );
}

#[tokio::test]
async fn a_body_just_under_the_limit_is_read() {
    let fake = serve(|_| Reply::stream(10, 1024 * 1024)).await;
    let connection = connection(
        IntegrationKind::OdooJsonRpc,
        &fake.base,
        Some("prod"),
        Some("admin"),
    );
    assert!(matches!(
        probe(&connection).await,
        Err(ConnectorFailure::Protocol(message)) if message.contains("JSON")
    ));
}

#[tokio::test]
async fn the_open_policy_reaches_the_fake_server() {
    let fake = serve(|_| Reply::json(200, &json!({}))).await;
    let connection = connection(IntegrationKind::OdooJson2, &fake.base, None, None);
    assert!(probe_with(OPEN, &connection).await.is_ok());
    assert!(fake.hits() > 0);
}
