#![allow(clippy::unwrap_used)]

mod fake;

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    mcp::{
        CodeExchange, RefreshExchange,
        pkce::{Pkce, challenge_of},
        ports::{McpGateway, McpOAuth},
    },
};
use capitainerie_integrations::{McpHttp, NetworkPolicy};
use fake::{GOOD_CODE, Options, start};
use serde_json::json;

const OPEN: NetworkPolicy = NetworkPolicy {
    allow_insecure_http: true,
    allow_private_networks: true,
};

const REDIRECT: &str = "https://console.example/connectors/oauth/callback";

fn adapter() -> McpHttp {
    McpHttp::new(OPEN).unwrap()
}

async fn granted(fake: &fake::Fake, adapter: &McpHttp) -> SecretToken {
    let server = format!("{}/mcp", fake.base);
    let discovery = adapter.discover(&server).await.unwrap();
    let client_id = adapter
        .register_client(
            &discovery.registration_endpoint,
            REDIRECT,
            &discovery.scopes,
        )
        .await
        .unwrap();
    let pkce = Pkce::generate().unwrap();
    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = browser
        .get(&discovery.authorization_endpoint)
        .query(&[
            ("response_type", "code"),
            ("client_id", client_id.as_str()),
            ("redirect_uri", REDIRECT),
            ("state", "s"),
            ("code_challenge", pkce.challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("resource", discovery.resource.as_str()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 302);
    let verifier = SecretToken::new(pkce.verifier);
    let tokens = adapter
        .exchange_code(&CodeExchange {
            token_endpoint: &discovery.token_endpoint,
            client_id: &client_id,
            resource: &discovery.resource,
            code: GOOD_CODE,
            code_verifier: &verifier,
            redirect_uri: REDIRECT,
        })
        .await
        .unwrap();
    tokens.access_token
}

#[tokio::test]
async fn discovery_follows_the_challenge_hint_to_the_authorization_server() {
    let fake = start(Options {
        hint_in_challenge: true,
        ..Options::default()
    })
    .await;

    let discovery = adapter()
        .discover(&format!("{}/mcp", fake.base))
        .await
        .unwrap();

    assert_eq!(discovery.resource, format!("{}/mcp", fake.base));
    assert_eq!(discovery.issuer, format!("{}/realms/mestier", fake.base));
    assert_eq!(
        discovery.token_endpoint,
        format!("{}/realms/mestier/token", fake.base)
    );
    assert_eq!(discovery.scopes, ["mestier.read", "mestier.doe.write"]);
    let probe = &fake.requests("POST", "/mcp")[0];
    assert_eq!(probe.authorization, None);
    assert_eq!(probe.json()["method"], "initialize");
    assert_eq!(
        fake.requests("GET", "/.well-known/oauth-protected-resource")
            .len(),
        1
    );
}

#[tokio::test]
async fn discovery_falls_back_to_the_well_known_paths_and_to_openid_configuration() {
    let fake = start(Options {
        hint_in_challenge: false,
        openid_only: true,
        ..Options::default()
    })
    .await;

    let discovery = adapter()
        .discover(&format!("{}/mcp", fake.base))
        .await
        .unwrap();

    assert_eq!(
        discovery.registration_endpoint,
        format!("{}/realms/mestier/clients/register", fake.base)
    );
    assert_eq!(
        fake.requests("GET", "/.well-known/oauth-protected-resource/mcp")
            .len(),
        1,
        "the path-suffixed location comes first and is missing here"
    );
    assert_eq!(
        fake.requests("GET", "/realms/mestier/.well-known/openid-configuration")
            .len(),
        1
    );
}

#[tokio::test]
async fn a_dynamic_registration_asks_for_a_public_pkce_client() {
    let fake = start(Options::default()).await;

    let client_id = adapter()
        .register_client(
            &format!("{}/realms/mestier/clients/register", fake.base),
            REDIRECT,
            &["mestier.read".into()],
        )
        .await
        .unwrap();

    assert_eq!(client_id, "client-xyz");
    let body = fake.requests("POST", "/realms/mestier/clients/register")[0].json();
    assert_eq!(body["client_name"], "Capitainerie");
    assert_eq!(body["redirect_uris"], json!([REDIRECT]));
    assert_eq!(body["token_endpoint_auth_method"], "none");
    assert_eq!(
        body["grant_types"],
        json!(["authorization_code", "refresh_token"])
    );
    assert_eq!(body["scope"], "mestier.read");
}

#[tokio::test]
async fn the_code_is_exchanged_with_pkce_and_the_resource_then_refreshed_with_rotation() {
    let fake = start(Options::default()).await;
    let adapter = adapter();
    let token = granted(&fake, &adapter).await;
    assert_eq!(token.expose(), "at-1");

    let exchange = &fake.requests("POST", "/realms/mestier/token")[0].form();
    assert_eq!(exchange["grant_type"], "authorization_code");
    assert_eq!(exchange["redirect_uri"], REDIRECT);
    assert_eq!(exchange["resource"], format!("{}/mcp", fake.base));
    assert_eq!(challenge_of(&exchange["code_verifier"]).len(), 43);

    let refresh = SecretToken::new("rt-1");
    let fresh = adapter
        .refresh(&RefreshExchange {
            token_endpoint: &format!("{}/realms/mestier/token", fake.base),
            client_id: "client-xyz",
            resource: &format!("{}/mcp", fake.base),
            refresh_token: &refresh,
        })
        .await
        .unwrap();
    assert_eq!(fresh.access_token.expose(), "at-2");
    assert_eq!(fresh.refresh_token.unwrap().expose(), "rt-2");
    assert!(fresh.expires_at.is_some());

    let replay = adapter
        .refresh(&RefreshExchange {
            token_endpoint: &format!("{}/realms/mestier/token", fake.base),
            client_id: "client-xyz",
            resource: &format!("{}/mcp", fake.base),
            refresh_token: &refresh,
        })
        .await
        .unwrap_err();
    assert!(matches!(replay, CoreError::Conflict(_)), "{replay}");
    assert!(!replay.to_string().contains("leak"));
}

#[tokio::test]
async fn a_wrong_verifier_is_refused_without_echoing_the_server_message() {
    let fake = start(Options::default()).await;
    let adapter = adapter();
    let discovery = adapter
        .discover(&format!("{}/mcp", fake.base))
        .await
        .unwrap();
    let verifier = SecretToken::new("not-the-verifier");

    let error = adapter
        .exchange_code(&CodeExchange {
            token_endpoint: &discovery.token_endpoint,
            client_id: "client-xyz",
            resource: &discovery.resource,
            code: "unknown",
            code_verifier: &verifier,
            redirect_uri: REDIRECT,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Conflict(_)) || matches!(error, CoreError::Invalid(_)));
    assert!(!error.to_string().contains("leak"));
    assert!(!error.to_string().contains("not-the-verifier"));
}

#[tokio::test]
async fn tools_are_listed_and_called_with_the_user_token() {
    let fake = start(Options::default()).await;
    let adapter = adapter();
    let token = granted(&fake, &adapter).await;
    let url = format!("{}/mcp", fake.base);

    let tools = adapter.list_tools(&url, &token).await.unwrap();

    let names: Vec<_> = tools.iter().map(|tool| tool.name.as_str()).collect();
    assert_eq!(names, ["list_doe", "write_doe", "bare"]);
    assert_eq!(tools[0].read_only_hint, Some(true));
    assert_eq!(tools[0].description.as_deref(), Some("Lister les DOE"));
    assert_eq!(tools[1].read_only_hint, Some(false));
    assert_eq!(tools[1].destructive_hint, Some(true));
    assert_eq!(tools[2].read_only_hint, None);
    assert_eq!(tools[0].input_schema["type"], "object");
    let authorized = fake
        .requests("POST", "/mcp")
        .into_iter()
        .filter(|seen| seen.authorization.is_some())
        .count();
    assert!(authorized >= 2);

    let output = adapter
        .call_tool(&url, &token, "list_doe", &json!({"q": 1}))
        .await
        .unwrap();
    assert_eq!(output.text, r#"called list_doe {"q":1}"#);
    assert!(!output.is_error);

    let big = adapter
        .call_tool(&url, &token, "big", &json!({}))
        .await
        .unwrap();
    assert_eq!(big.text.chars().count(), 12_001);
    let failed = adapter
        .call_tool(&url, &token, "boom", &json!({}))
        .await
        .unwrap();
    assert!(failed.is_error);
    assert_eq!(failed.text, "échec");
    let invalid = adapter
        .call_tool(&url, &token, "list_doe", &json!([1]))
        .await
        .unwrap_err();
    assert!(matches!(invalid, CoreError::Invalid(_)));
}

#[tokio::test]
async fn a_rejected_token_asks_to_reconnect_without_echoing_it() {
    let fake = start(Options::default()).await;
    let adapter = adapter();
    let token = granted(&fake, &adapter).await;
    fake.expire_access_token();

    let error = adapter
        .list_tools(&format!("{}/mcp", fake.base), &token)
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Conflict(_)), "{error}");
    assert!(!error.to_string().contains(token.expose()));
}

#[tokio::test]
async fn the_network_policy_blocks_private_targets_before_any_request() {
    let fake = start(Options::default()).await;
    let strict = McpHttp::new(NetworkPolicy::default()).unwrap();
    let url = format!("{}/mcp", fake.base);

    let discovery = strict.discover(&url).await.unwrap_err();
    let tools = strict
        .list_tools(&url, &SecretToken::new("t"))
        .await
        .unwrap_err();
    let registration = strict
        .register_client(
            &format!("{}/realms/mestier/clients/register", fake.base),
            REDIRECT,
            &[],
        )
        .await
        .unwrap_err();

    for error in [discovery, tools, registration] {
        assert!(matches!(error, CoreError::Invalid(_)), "{error}");
        assert!(error.to_string().contains("politique réseau"), "{error}");
    }
    assert_eq!(fake.total_requests(), 0);
}

#[tokio::test]
async fn plain_http_needs_its_own_flag() {
    let fake = start(Options::default()).await;
    let private_only = McpHttp::new(NetworkPolicy {
        allow_insecure_http: false,
        allow_private_networks: true,
    })
    .unwrap();

    let error = private_only
        .discover(&format!("{}/mcp", fake.base))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("politique réseau"), "{error}");
    assert_eq!(fake.total_requests(), 0);
}
