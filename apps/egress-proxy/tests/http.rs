#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use capitainerie_common::{OrganizationId, RunId};
use capitainerie_core::domain::{
    agent::SecretToken,
    execution::{RepositoryRef, TaskGrant},
};
use capitainerie_egress_proxy::{Credentials, CredentialsError, Proxy, ProxyConfig, router};
use chrono::Utc;
use futures_util::stream;
use tokio::{net::TcpListener, sync::mpsc};

const TASK_TOKEN: &str = "task-token-0001";
const ANTHROPIC_KEY: &str = "sk-ant-real-key";
const INSTALLATION_TOKEN: &str = "ghs_installation";
const BRANCH: &str = "capitainerie/0a1b2c3d";
const OLD: &str = "1111111111111111111111111111111111111111";
const NEW: &str = "2222222222222222222222222222222222222222";
const ZERO: &str = "0000000000000000000000000000000000000000";

fn sha256(token: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(
        aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, token.as_bytes()).as_ref(),
    );
    out
}

fn grant() -> TaskGrant {
    TaskGrant {
        run_id: RunId::new(),
        organization_id: OrganizationId(uuid::Uuid::nil()),
        repository: "acme/web".parse().unwrap(),
        branch: BRANCH.into(),
        expires_at: Utc::now() + chrono::Duration::minutes(30),
        revoked_at: None,
    }
}

struct Fake {
    grants: HashMap<[u8; 32], TaskGrant>,
    anthropic_key: Option<&'static str>,
    github_token: Option<&'static str>,
    lookups: Arc<AtomicUsize>,
}

impl Fake {
    fn with(grant: TaskGrant) -> Self {
        Self {
            grants: HashMap::from([(sha256(TASK_TOKEN), grant)]),
            anthropic_key: Some(ANTHROPIC_KEY),
            github_token: Some(INSTALLATION_TOKEN),
            lookups: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl Credentials for Fake {
    async fn grant(&self, token_sha256: [u8; 32]) -> Result<Option<TaskGrant>, CredentialsError> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        Ok(self.grants.get(&token_sha256).cloned())
    }

    async fn anthropic_key(
        &self,
        _: OrganizationId,
    ) -> Result<Option<SecretToken>, CredentialsError> {
        Ok(self.anthropic_key.map(SecretToken::new))
    }

    async fn github_token(
        &self,
        _: OrganizationId,
        _: &RepositoryRef,
    ) -> Result<Option<SecretToken>, CredentialsError> {
        Ok(self.github_token.map(SecretToken::new))
    }
}

#[derive(Debug, Clone)]
struct Captured {
    method: Method,
    uri: String,
    headers: HeaderMap,
    body: Bytes,
}

type Log = Arc<Mutex<Vec<Captured>>>;

async fn capture(State(log): State<Log>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    log.lock().unwrap().push(Captured {
        method: parts.method,
        uri: parts.uri.to_string(),
        headers: parts.headers,
        body,
    });
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-test")
        .body(Body::from("upstream-ok"))
        .unwrap()
}

async fn serve(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

async fn capturing_upstream() -> (String, Log) {
    let log = Log::default();
    let app = Router::new().fallback(capture).with_state(log.clone());
    (serve(app).await, log)
}

struct Harness {
    url: String,
    anthropic: Log,
    github: Log,
    client: reqwest::Client,
    lookups: Arc<AtomicUsize>,
}

fn plain_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

async fn harness_with(fake: Fake) -> Harness {
    let (anthropic_url, anthropic) = capturing_upstream().await;
    let (github_url, github) = capturing_upstream().await;
    start(fake, anthropic_url, github_url, anthropic, github).await
}

async fn start(
    fake: Fake,
    anthropic_url: String,
    github_url: String,
    anthropic: Log,
    github: Log,
) -> Harness {
    let lookups = fake.lookups.clone();
    let proxy = Proxy::with_http(
        fake,
        ProxyConfig {
            anthropic_base: anthropic_url,
            github_base: github_url,
        },
        plain_client(),
    );
    Harness {
        url: serve(router(proxy)).await,
        anthropic,
        github,
        client: plain_client(),
        lookups,
    }
}

async fn harness() -> Harness {
    harness_with(Fake::with(grant())).await
}

fn basic(token: &str) -> String {
    format!(
        "Basic {}",
        STANDARD.encode(format!("x-access-token:{token}"))
    )
}

fn pkt(payload: &[u8]) -> Vec<u8> {
    let mut out = format!("{:04x}", payload.len() + 4).into_bytes();
    out.extend_from_slice(payload);
    out
}

fn push_body(commands: &[(&str, &str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, (old, new, refname)) in commands.iter().enumerate() {
        let mut line = format!("{old} {new} {refname}").into_bytes();
        if index == 0 {
            line.extend_from_slice(b"\0report-status agent=git/2.45");
        }
        out.extend(pkt(&line));
    }
    out.extend_from_slice(b"0000PACK-the-pack-data");
    out
}

fn branch_ref() -> String {
    format!("refs/heads/{BRANCH}")
}

impl Harness {
    async fn messages(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .post(format!("{}/anthropic{path}", self.url))
            .header("x-api-key", token)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "tools-2024")
            .header("content-type", "application/json")
            .body(r#"{"model":"m"}"#)
            .send()
            .await
            .unwrap()
    }

    async fn git_get(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .get(format!("{}/git/{path}", self.url))
            .header("authorization", basic(token))
            .send()
            .await
            .unwrap()
    }

    async fn git_post(&self, path: &str, token: &str, body: Vec<u8>) -> reqwest::Response {
        self.client
            .post(format!("{}/git/{path}", self.url))
            .header("authorization", basic(token))
            .header("content-type", "application/x-git-receive-pack-request")
            .body(body)
            .send()
            .await
            .unwrap()
    }

    fn received(log: &Log) -> Vec<Captured> {
        log.lock().unwrap().clone()
    }
}

#[tokio::test]
async fn healthz_answers_and_unknown_routes_are_404() {
    let h = harness().await;
    let ok = h
        .client
        .get(format!("{}/healthz", h.url))
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    let other = h
        .client
        .get(format!("{}/elsewhere", h.url))
        .send()
        .await
        .unwrap();
    assert_eq!(other.status(), 404);
}

#[tokio::test]
async fn an_unknown_token_is_401_with_the_same_body_as_a_revoked_one() {
    let mut revoked = grant();
    revoked.revoked_at = Some(Utc::now());
    let h = harness_with(Fake::with(revoked)).await;

    let unknown = h.messages("/v1/messages", "who-knows").await;
    let unknown_status = unknown.status();
    let unknown_body = unknown.text().await.unwrap();
    let refused = h.messages("/v1/messages", TASK_TOKEN).await;
    assert_eq!(unknown_status, 401);
    assert_eq!(refused.status(), 401);
    assert_eq!(refused.text().await.unwrap(), unknown_body);
    assert!(!unknown_body.contains("who-knows"));
    assert!(Harness::received(&h.anthropic).is_empty());

    let git = h
        .git_get(
            "acme/web.git/info/refs?service=git-upload-pack",
            "who-knows",
        )
        .await;
    assert_eq!(git.status(), 401);
}

#[tokio::test]
async fn a_request_without_credentials_is_401() {
    let h = harness().await;
    let anthropic = h
        .client
        .post(format!("{}/anthropic/v1/messages", h.url))
        .send()
        .await
        .unwrap();
    assert_eq!(anthropic.status(), 401);
    let git = h
        .client
        .get(format!(
            "{}/git/acme/web.git/info/refs?service=git-upload-pack",
            h.url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(git.status(), 401);
}

#[tokio::test]
async fn git_basic_auth_needs_the_access_token_user() {
    let h = harness().await;
    let response = h
        .client
        .get(format!(
            "{}/git/acme/web.git/info/refs?service=git-upload-pack",
            h.url
        ))
        .header(
            "authorization",
            format!("Basic {}", STANDARD.encode(format!("bob:{TASK_TOKEN}"))),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn spec_exec_6_a_revoked_or_expired_token_is_refused() {
    for mutate in [
        (|g: &mut TaskGrant| g.revoked_at = Some(Utc::now())) as fn(&mut TaskGrant),
        |g| g.expires_at = Utc::now() - chrono::Duration::seconds(1),
    ] {
        let mut dead = grant();
        mutate(&mut dead);
        let h = harness_with(Fake::with(dead)).await;
        assert_eq!(h.messages("/v1/messages", TASK_TOKEN).await.status(), 401);
        let git = h
            .git_get("acme/web.git/info/refs?service=git-upload-pack", TASK_TOKEN)
            .await;
        assert_eq!(git.status(), 401);
        assert!(Harness::received(&h.anthropic).is_empty());
        assert!(Harness::received(&h.github).is_empty());
    }
}

#[tokio::test]
async fn a_valid_lookup_is_reused_briefly() {
    let h = harness().await;
    for _ in 0..3 {
        assert_eq!(h.messages("/v1/messages", TASK_TOKEN).await.status(), 200);
    }
    assert_eq!(h.lookups.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn anthropic_messages_are_forwarded_with_the_real_key_only() {
    let h = harness().await;
    let response = h.messages("/v1/messages?beta=true", TASK_TOKEN).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "upstream-ok");

    let seen = Harness::received(&h.anthropic);
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.method, Method::POST);
    assert_eq!(request.uri, "/v1/messages?beta=true");
    assert_eq!(request.headers["x-api-key"], ANTHROPIC_KEY);
    assert_eq!(request.headers["anthropic-version"], "2023-06-01");
    assert_eq!(request.headers["anthropic-beta"], "tools-2024");
    assert_eq!(request.body, Bytes::from_static(br#"{"model":"m"}"#));
    assert!(request.headers.get("authorization").is_none());
    for (_, value) in &request.headers {
        assert!(!value.to_str().unwrap_or_default().contains(TASK_TOKEN));
    }
}

#[tokio::test]
async fn a_bearer_task_token_and_count_tokens_are_accepted() {
    let h = harness().await;
    let response = h
        .client
        .post(format!("{}/anthropic/v1/messages/count_tokens", h.url))
        .bearer_auth(TASK_TOKEN)
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let seen = Harness::received(&h.anthropic);
    assert_eq!(seen[0].uri, "/v1/messages/count_tokens");
    assert_eq!(seen[0].headers["x-api-key"], ANTHROPIC_KEY);
    assert!(seen[0].headers.get("authorization").is_none());
}

#[tokio::test]
async fn other_anthropic_paths_and_methods_are_403() {
    let h = harness().await;
    for path in [
        "/v1/complete",
        "/v1/models",
        "/v1/messages/batches",
        "/v1/organizations/me",
        "/v1/messages/",
    ] {
        let response = h.messages(path, TASK_TOKEN).await;
        assert_eq!(response.status(), 403, "{path}");
        assert!(!response.text().await.unwrap().contains(TASK_TOKEN));
    }
    let get = h
        .client
        .get(format!("{}/anthropic/v1/messages", h.url))
        .header("x-api-key", TASK_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 403);
    assert!(Harness::received(&h.anthropic).is_empty());
}

#[tokio::test]
async fn an_organization_without_an_anthropic_key_gets_a_clear_403() {
    let mut fake = Fake::with(grant());
    fake.anthropic_key = None;
    let h = harness_with(fake).await;
    let response = h.messages("/v1/messages", TASK_TOKEN).await;
    assert_eq!(response.status(), 403);
    assert!(response.text().await.unwrap().contains("Anthropic key"));
    assert!(Harness::received(&h.anthropic).is_empty());
}

#[tokio::test]
async fn upstream_responses_are_streamed_back_as_they_arrive() {
    let (tx, rx) = mpsc::channel::<Bytes>(4);
    let rx = Arc::new(tokio::sync::Mutex::new(Some(rx)));
    let upstream = Router::new()
        .fallback(
            |State(rx): State<Arc<tokio::sync::Mutex<Option<mpsc::Receiver<Bytes>>>>>| async move {
                let rx = rx.lock().await.take().unwrap();
                let body = Body::from_stream(stream::unfold(rx, |mut rx| async {
                    rx.recv()
                        .await
                        .map(|chunk| (Ok::<_, Infallible>(chunk), rx))
                }));
                Response::builder()
                    .header("content-type", "text/event-stream")
                    .body(body)
                    .unwrap()
            },
        )
        .with_state(rx);
    let anthropic_url = serve(upstream).await;
    let (github_url, github) = capturing_upstream().await;
    let h = start(
        Fake::with(grant()),
        anthropic_url,
        github_url,
        Log::default(),
        github,
    )
    .await;

    let mut response = h.messages("/v1/messages", TASK_TOKEN).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/event-stream");

    tx.send(Bytes::from_static(b"data: one\n\n")).await.unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), response.chunk())
        .await
        .expect("the first event must arrive before the upstream finishes")
        .unwrap()
        .unwrap();
    assert_eq!(first, Bytes::from_static(b"data: one\n\n"));

    tx.send(Bytes::from_static(b"data: two\n\n")).await.unwrap();
    drop(tx);
    let mut rest = Vec::new();
    while let Some(chunk) = response.chunk().await.unwrap() {
        rest.extend_from_slice(&chunk);
    }
    assert_eq!(rest, b"data: two\n\n");
}

#[tokio::test]
async fn upstream_error_statuses_pass_through() {
    let upstream =
        Router::new().fallback(|| async { (StatusCode::TOO_MANY_REQUESTS, "slow down") });
    let anthropic_url = serve(upstream).await;
    let (github_url, github) = capturing_upstream().await;
    let h = start(
        Fake::with(grant()),
        anthropic_url,
        github_url,
        Log::default(),
        github,
    )
    .await;
    let response = h.messages("/v1/messages", TASK_TOKEN).await;
    assert_eq!(response.status(), 429);
    assert_eq!(response.text().await.unwrap(), "slow down");
}

#[tokio::test]
async fn git_fetch_is_forwarded_with_the_installation_token() {
    let h = harness().await;
    let refs = h
        .git_get("acme/web.git/info/refs?service=git-upload-pack", TASK_TOKEN)
        .await;
    assert_eq!(refs.status(), 200);
    assert_eq!(refs.text().await.unwrap(), "upstream-ok");
    let pack = h
        .git_post(
            "acme/web.git/git-upload-pack",
            TASK_TOKEN,
            b"0032want abc\n0000".to_vec(),
        )
        .await;
    assert_eq!(pack.status(), 200);

    let seen = Harness::received(&h.github);
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].method, Method::GET);
    assert_eq!(
        seen[0].uri,
        "/acme/web.git/info/refs?service=git-upload-pack"
    );
    assert_eq!(seen[1].uri, "/acme/web.git/git-upload-pack");
    assert_eq!(seen[1].body, Bytes::from_static(b"0032want abc\n0000"));
    for request in &seen {
        assert_eq!(
            request.headers["authorization"],
            basic(INSTALLATION_TOKEN).as_str()
        );
        for (_, value) in &request.headers {
            assert!(!value.to_str().unwrap_or_default().contains(TASK_TOKEN));
        }
    }
}

#[tokio::test]
async fn spec_exec_5_another_repository_is_403() {
    let h = harness().await;
    for path in [
        "acme/autre.git/info/refs?service=git-upload-pack",
        "other/web.git/info/refs?service=git-upload-pack",
        "acme/web/info/refs?service=git-upload-pack",
        "acme/web.git.git/info/refs?service=git-upload-pack",
    ] {
        let response = h.git_get(path, TASK_TOKEN).await;
        assert_eq!(response.status(), 403, "{path}");
    }
    let push = h
        .git_post(
            "acme/autre.git/git-receive-pack",
            TASK_TOKEN,
            push_body(&[(OLD, NEW, &branch_ref())]),
        )
        .await;
    assert_eq!(push.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
}

#[tokio::test]
async fn spec_exec_5_a_push_to_the_execution_branch_passes() {
    let h = harness().await;
    let body = push_body(&[(OLD, NEW, &branch_ref())]);
    let response = h
        .git_post("acme/web.git/git-receive-pack", TASK_TOKEN, body.clone())
        .await;
    assert_eq!(response.status(), 200);
    let seen = Harness::received(&h.github);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].uri, "/acme/web.git/git-receive-pack");
    assert_eq!(seen[0].body.as_ref(), body.as_slice());
    assert_eq!(
        seen[0].headers["authorization"],
        basic(INSTALLATION_TOKEN).as_str()
    );
}

#[tokio::test]
async fn spec_exec_5_a_push_arriving_in_small_chunks_passes_intact() {
    let h = harness().await;
    let body = push_body(&[(ZERO, NEW, &branch_ref())]);
    let chunks: Vec<Result<Vec<u8>, Infallible>> =
        body.chunks(7).map(|chunk| Ok(chunk.to_vec())).collect();
    let response = h
        .client
        .post(format!("{}/git/acme/web.git/git-receive-pack", h.url))
        .header("authorization", basic(TASK_TOKEN))
        .body(reqwest::Body::wrap_stream(stream::iter(chunks)))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let seen = Harness::received(&h.github);
    assert_eq!(seen[0].body.as_ref(), body.as_slice());
}

#[tokio::test]
async fn spec_exec_5_a_push_to_main_is_403() {
    let h = harness().await;
    let response = h
        .git_post(
            "acme/web.git/git-receive-pack",
            TASK_TOKEN,
            push_body(&[(OLD, NEW, "refs/heads/main")]),
        )
        .await;
    assert_eq!(response.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
}

#[tokio::test]
async fn a_ref_deletion_is_403() {
    let h = harness().await;
    let response = h
        .git_post(
            "acme/web.git/git-receive-pack",
            TASK_TOKEN,
            push_body(&[(OLD, ZERO, &branch_ref())]),
        )
        .await;
    assert_eq!(response.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
}

#[tokio::test]
async fn one_bad_command_among_several_is_403() {
    let h = harness().await;
    let response = h
        .git_post(
            "acme/web.git/git-receive-pack",
            TASK_TOKEN,
            push_body(&[(OLD, NEW, &branch_ref()), (OLD, NEW, "refs/heads/main")]),
        )
        .await;
    assert_eq!(response.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
}

#[tokio::test]
async fn an_unreadable_push_is_403() {
    let h = harness().await;
    for body in [b"garbage".to_vec(), b"0000".to_vec(), Vec::new()] {
        let response = h
            .git_post("acme/web.git/git-receive-pack", TASK_TOKEN, body)
            .await;
        assert_eq!(response.status(), 403);
    }
    let encoded = h
        .client
        .post(format!("{}/git/acme/web.git/git-receive-pack", h.url))
        .header("authorization", basic(TASK_TOKEN))
        .header("content-encoding", "gzip")
        .body(push_body(&[(OLD, NEW, &branch_ref())]))
        .send()
        .await
        .unwrap();
    assert_eq!(encoded.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
}

#[tokio::test]
async fn other_git_requests_are_403() {
    let h = harness().await;
    for path in [
        "acme/web.git/HEAD",
        "acme/web.git/objects/info/packs",
        "acme/web.git/info/refs",
        "acme/web.git/info/refs?service=git-upload-archive",
        "acme/web.git/info/refs?service=git-upload-pack&x=1",
    ] {
        assert_eq!(h.git_get(path, TASK_TOKEN).await.status(), 403, "{path}");
    }
    let post = h
        .git_post("acme/web.git/git-upload-archive", TASK_TOKEN, Vec::new())
        .await;
    assert_eq!(post.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
}

#[tokio::test]
async fn an_organization_without_github_gets_a_403() {
    let mut fake = Fake::with(grant());
    fake.github_token = None;
    let h = harness_with(fake).await;
    let response = h
        .git_get("acme/web.git/info/refs?service=git-upload-pack", TASK_TOKEN)
        .await;
    assert_eq!(response.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
}

#[tokio::test]
async fn an_unreachable_upstream_is_a_generic_502() {
    let fake = Fake::with(grant());
    let dead = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let h = start(fake, dead.clone(), dead, Log::default(), Log::default()).await;
    let response = h.messages("/v1/messages", TASK_TOKEN).await;
    assert_eq!(response.status(), 502);
    let body = response.text().await.unwrap();
    assert!(!body.contains("127.0.0.1"));
}
