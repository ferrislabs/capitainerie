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
    execution::{ExecutionMode, RepositoryRef, TaskGrant},
    provider::ProviderKind,
};
use capitainerie_egress_proxy::{Credentials, CredentialsError, Proxy, ProxyConfig, router};
use chrono::Utc;
use futures_util::stream;
use tokio::{net::TcpListener, sync::mpsc};

const TASK_TOKEN: &str = "task-token-0001";
const ANTHROPIC_KEY: &str = "sk-ant-real-key";
const OPENAI_KEY: &str = "sk-openai-real-key";
const GOOGLE_KEY: &str = "AIza-real-key";
const CODEX_MODEL: &str = "gpt-6-luna";
const AGY_MODEL: &str = "gemini-3.1-pro";
const AGY_WIRE: &str = "gemini-3.1-pro-preview";
const HELPER_WIRE: &str = "gemini-3.1-flash-lite-preview";
const AGY_BODY: &str = include_str!("fixtures/agy-request-body.json");
const INSTALLATION_TOKEN: &str = "ghs_installation";
const READ_ONLY_TOKEN: &str = "ghs_read_only";
const INITIATOR: &str = "user-1";
const INITIATOR_KEY: &str = "sk-ant-initiator-key";
const MODEL: &str = "claude-sonnet-5-5";
const BRANCH: &str = "capitainerie/0a1b2c3d4e5f";
const OLD: &str = "1111111111111111111111111111111111111111";
const NEW: &str = "2222222222222222222222222222222222222222";
const MESSAGE: &str = r#"{"model":"claude-sonnet-5-5","max_tokens":1024}"#;
const ZERO: &str = "0000000000000000000000000000000000000000";

fn sha256(token: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(
        aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA256, token.as_bytes()).as_ref(),
    );
    out
}

fn codex_grant() -> TaskGrant {
    TaskGrant {
        model: CODEX_MODEL.into(),
        ..grant()
    }
}

fn agy_grant() -> TaskGrant {
    TaskGrant {
        model: AGY_MODEL.into(),
        ..grant()
    }
}

fn grant() -> TaskGrant {
    TaskGrant {
        run_id: RunId::new(),
        organization_id: OrganizationId(uuid::Uuid::nil()),
        repository: "acme/web".parse().unwrap(),
        branch: BRANCH.into(),
        model: MODEL.into(),
        mode: ExecutionMode::Change,
        initiated_by: Some(INITIATOR.into()),
        expires_at: Utc::now() + chrono::Duration::minutes(30),
        revoked_at: None,
    }
}

struct Fake {
    grants: HashMap<[u8; 32], TaskGrant>,
    anthropic_key: Option<&'static str>,
    openai_key: Option<&'static str>,
    google_key: Option<&'static str>,
    user_keys: HashMap<&'static str, &'static str>,
    github_token: Option<&'static str>,
    lookups: Arc<AtomicUsize>,
    token_modes: Arc<Mutex<Vec<ExecutionMode>>>,
}

impl Fake {
    fn with(grant: TaskGrant) -> Self {
        Self {
            grants: HashMap::from([(sha256(TASK_TOKEN), grant)]),
            anthropic_key: Some(ANTHROPIC_KEY),
            openai_key: Some(OPENAI_KEY),
            google_key: Some(GOOGLE_KEY),
            user_keys: HashMap::new(),
            github_token: Some(INSTALLATION_TOKEN),
            lookups: Arc::new(AtomicUsize::new(0)),
            token_modes: Arc::default(),
        }
    }
}

impl Credentials for Fake {
    async fn grant(&self, token_sha256: [u8; 32]) -> Result<Option<TaskGrant>, CredentialsError> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        Ok(self.grants.get(&token_sha256).cloned())
    }

    async fn executor_key(
        &self,
        provider: ProviderKind,
        _: OrganizationId,
        initiated_by: Option<&str>,
    ) -> Result<Option<SecretToken>, CredentialsError> {
        let key = match provider {
            ProviderKind::Anthropic => {
                let own = initiated_by.and_then(|subject| self.user_keys.get(subject).copied());
                own.or(self.anthropic_key)
            }
            ProviderKind::OpenAi => self.openai_key,
            ProviderKind::Google => self.google_key,
            _ => None,
        };
        Ok(key.map(SecretToken::new))
    }

    async fn github_token(
        &self,
        _: OrganizationId,
        _: &RepositoryRef,
        mode: ExecutionMode,
    ) -> Result<Option<SecretToken>, CredentialsError> {
        self.token_modes.lock().unwrap().push(mode);
        Ok(self.github_token.map(|token| match mode {
            ExecutionMode::Change => SecretToken::new(token),
            ExecutionMode::Audit => SecretToken::new(READ_ONLY_TOKEN),
        }))
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
    openai: Log,
    google: Log,
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

async fn start_openai(fake: Fake, openai_url: String, openai: Log) -> Harness {
    let (anthropic_url, anthropic) = capturing_upstream().await;
    let (github_url, github) = capturing_upstream().await;
    let mut h = start_with(
        fake,
        anthropic_url,
        openai_url,
        github_url,
        anthropic,
        github,
        capturing_upstream().await,
    )
    .await;
    h.openai = openai;
    h
}

async fn start(
    fake: Fake,
    anthropic_url: String,
    github_url: String,
    anthropic: Log,
    github: Log,
) -> Harness {
    let (openai_url, openai) = capturing_upstream().await;
    let mut h = start_with(
        fake,
        anthropic_url,
        openai_url,
        github_url,
        anthropic,
        github,
        capturing_upstream().await,
    )
    .await;
    h.openai = openai;
    h
}

async fn start_with(
    fake: Fake,
    anthropic_url: String,
    openai_url: String,
    github_url: String,
    anthropic: Log,
    github: Log,
    google: (String, Log),
) -> Harness {
    let lookups = fake.lookups.clone();
    let (google_url, google) = google;
    let proxy = Proxy::with_http(
        fake,
        ProxyConfig {
            anthropic_base: anthropic_url,
            openai_base: openai_url,
            google_base: google_url,
            github_base: github_url,
        },
        plain_client(),
    );
    Harness {
        url: serve(router(proxy)).await,
        anthropic,
        openai: Log::default(),
        google,
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
            .header("anthropic-beta", "prompt-caching-2024-07-31, tools-2024")
            .header("content-type", "application/json")
            .body(MESSAGE)
            .send()
            .await
            .unwrap()
    }

    async fn messages_with(&self, body: serde_json::Value) -> reqwest::Response {
        self.client
            .post(format!("{}/anthropic/v1/messages", self.url))
            .header("x-api-key", TASK_TOKEN)
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .unwrap()
    }

    async fn responses(&self, token: &str, body: serde_json::Value) -> reqwest::Response {
        self.client
            .post(format!("{}/openai/v1/responses", self.url))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .header("openai-organization", "org-container")
            .header("openai-project", "proj-container")
            .body(body.to_string())
            .send()
            .await
            .unwrap()
    }

    async fn generate(
        &self,
        path: &str,
        token_header: (&str, &str),
        body: &str,
    ) -> reqwest::Response {
        self.client
            .post(format!("{}/google{path}", self.url))
            .header(token_header.0, token_header.1)
            .header("content-type", "application/json")
            .header("accept-encoding", "x-agy-coding")
            .header("x-goog-api-client", "gl-go/1.0 container-token")
            .header("user-agent", "agy/1.5.3")
            .body(body.to_owned())
            .send()
            .await
            .unwrap()
    }

    async fn agy(&self, body: &str) -> reqwest::Response {
        self.generate(&stream_path(AGY_WIRE), ("x-goog-api-key", TASK_TOKEN), body)
            .await
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
async fn git_without_credentials_gets_a_basic_challenge() {
    let h = harness().await;

    let response = h
        .client
        .get(format!(
            "{}/git/acme/web.git/info/refs?service=git-upload-pack",
            h.url
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
    assert_eq!(
        response.headers()["www-authenticate"],
        r#"Basic realm="capitainerie""#
    );
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
    assert_eq!(
        request.headers["anthropic-beta"],
        "prompt-caching-2024-07-31"
    );
    assert_eq!(request.body, Bytes::from_static(MESSAGE.as_bytes()));
    assert!(request.headers.get("authorization").is_none());
    for (_, value) in &request.headers {
        assert!(!value.to_str().unwrap_or_default().contains(TASK_TOKEN));
    }
}

#[tokio::test]
async fn an_oauth_token_is_forwarded_as_a_bearer_with_the_oauth_beta() {
    const OAUTH_TOKEN: &str = "sk-ant-oat01-real-token";
    let mut fake = Fake::with(grant());
    fake.anthropic_key = Some(OAUTH_TOKEN);
    let h = harness_with(fake).await;
    assert_eq!(h.messages("/v1/messages", TASK_TOKEN).await.status(), 200);

    let seen = Harness::received(&h.anthropic);
    let request = &seen[0];
    assert_eq!(
        request.headers["authorization"],
        format!("Bearer {OAUTH_TOKEN}")
    );
    assert_eq!(
        request.headers["anthropic-beta"],
        "prompt-caching-2024-07-31,oauth-2025-04-20"
    );
    assert!(request.headers.get("x-api-key").is_none());
}

#[tokio::test]
async fn a_bearer_task_token_and_count_tokens_are_accepted() {
    let h = harness().await;
    let response = h
        .client
        .post(format!("{}/anthropic/v1/messages/count_tokens", h.url))
        .bearer_auth(TASK_TOKEN)
        .body(r#"{"model":"claude-sonnet-5-5"}"#)
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

fn read_only() -> Fake {
    Fake::with(TaskGrant {
        mode: ExecutionMode::Audit,
        ..grant()
    })
}

#[tokio::test]
async fn a_read_only_run_fetches_with_a_read_only_token() {
    let fake = read_only();
    let modes = fake.token_modes.clone();
    let h = harness_with(fake).await;
    let refs = h
        .git_get("acme/web.git/info/refs?service=git-upload-pack", TASK_TOKEN)
        .await;
    assert_eq!(refs.status(), 200);
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
    for request in &seen {
        assert_eq!(
            request.headers["authorization"],
            basic(READ_ONLY_TOKEN).as_str()
        );
    }
    assert_eq!(
        *modes.lock().unwrap(),
        [ExecutionMode::Audit, ExecutionMode::Audit]
    );
}

#[tokio::test]
async fn a_read_only_run_cannot_push_at_all() {
    let fake = read_only();
    let modes = fake.token_modes.clone();
    let h = harness_with(fake).await;
    let advertised = h
        .git_get(
            "acme/web.git/info/refs?service=git-receive-pack",
            TASK_TOKEN,
        )
        .await;
    assert_eq!(advertised.status(), 403);
    assert!(advertised.text().await.unwrap().contains("read-only"));
    let push = h
        .git_post(
            "acme/web.git/git-receive-pack",
            TASK_TOKEN,
            push_body(&[(OLD, NEW, &branch_ref())]),
        )
        .await;
    assert_eq!(push.status(), 403);
    assert!(Harness::received(&h.github).is_empty());
    assert!(modes.lock().unwrap().is_empty(), "no token was minted");
}

#[tokio::test]
async fn a_change_run_asks_for_a_writing_token() {
    let fake = Fake::with(grant());
    let modes = fake.token_modes.clone();
    let h = harness_with(fake).await;
    let response = h
        .git_post(
            "acme/web.git/git-receive-pack",
            TASK_TOKEN,
            push_body(&[(OLD, NEW, &branch_ref())]),
        )
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(*modes.lock().unwrap(), [ExecutionMode::Change]);
}

#[tokio::test]
async fn the_initiators_own_anthropic_key_comes_before_the_organizations() {
    let mut fake = Fake::with(grant());
    fake.user_keys.insert(INITIATOR, INITIATOR_KEY);
    let h = harness_with(fake).await;
    let response = h.messages("/v1/messages", TASK_TOKEN).await;
    assert_eq!(response.status(), 200);
    let seen = Harness::received(&h.anthropic);
    assert_eq!(seen[0].headers["x-api-key"], INITIATOR_KEY);

    let mut someone_else = Fake::with(grant());
    someone_else.user_keys.insert("user-2", INITIATOR_KEY);
    let h = harness_with(someone_else).await;
    assert_eq!(h.messages("/v1/messages", TASK_TOKEN).await.status(), 200);
    assert_eq!(
        Harness::received(&h.anthropic)[0].headers["x-api-key"],
        ANTHROPIC_KEY
    );

    let mut legacy = Fake::with(TaskGrant {
        initiated_by: None,
        ..grant()
    });
    legacy.user_keys.insert(INITIATOR, INITIATOR_KEY);
    let h = harness_with(legacy).await;
    assert_eq!(h.messages("/v1/messages", TASK_TOKEN).await.status(), 200);
    assert_eq!(
        Harness::received(&h.anthropic)[0].headers["x-api-key"],
        ANTHROPIC_KEY
    );
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

#[tokio::test]
async fn the_original_bytes_are_forwarded_not_a_reserialization() {
    let h = harness().await;
    let body = "{ \"max_tokens\" :  1,   \"model\":\"claude-sonnet-5-5\" ,\"x\":1.0}";
    let response = h
        .client
        .post(format!("{}/anthropic/v1/messages", h.url))
        .header("x-api-key", TASK_TOKEN)
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        Harness::received(&h.anthropic)[0].body.as_ref(),
        body.as_bytes()
    );
}

#[tokio::test]
async fn s1_a_model_other_than_the_grants_is_403() {
    let h = harness().await;
    for body in [
        serde_json::json!({"model": "claude-opus-5", "max_tokens": 10}),
        serde_json::json!({"max_tokens": 10}),
    ] {
        assert_eq!(h.messages_with(body).await.status(), 403);
    }
    assert!(Harness::received(&h.anthropic).is_empty());
}

#[tokio::test]
async fn s1_max_tokens_is_capped_at_128000() {
    let h = harness().await;
    let over = h
        .messages_with(serde_json::json!({"model": MODEL, "max_tokens": 128_001}))
        .await;
    let at = h
        .messages_with(serde_json::json!({"model": MODEL, "max_tokens": 128_000}))
        .await;
    assert_eq!(over.status(), 403);
    assert_eq!(at.status(), 200);
}

#[tokio::test]
async fn s1_remote_mcp_servers_and_containers_are_403() {
    let h = harness().await;
    for extra in [
        serde_json::json!({"mcp_servers": [{"type": "url", "url": "https://x", "name": "x"}]}),
        serde_json::json!({"container": "c1"}),
    ] {
        let mut body = serde_json::json!({"model": MODEL, "max_tokens": 10});
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert_eq!(h.messages_with(body).await.status(), 403);
    }
    assert!(Harness::received(&h.anthropic).is_empty());
}

#[tokio::test]
async fn s1_server_side_tools_are_403_and_client_tools_pass() {
    let h = harness().await;
    for kind in [
        "web_search_20250305",
        "web_fetch_20250910",
        "code_execution_20250522",
        "bash_20250124",
        "computer_20250124",
        "text_editor_20250728",
        "mcp_toolset",
    ] {
        let body = serde_json::json!({
            "model": MODEL, "max_tokens": 10,
            "tools": [{"type": kind, "name": "t"}],
        });
        assert_eq!(h.messages_with(body).await.status(), 403, "{kind}");
    }
    assert!(Harness::received(&h.anthropic).is_empty());
    let client_tools = serde_json::json!({
        "model": MODEL, "max_tokens": 10,
        "tools": [{"name": "Bash", "description": "d", "input_schema": {"type": "object"}}],
    });
    assert_eq!(h.messages_with(client_tools).await.status(), 200);
}

#[tokio::test]
async fn s1_only_allow_listed_betas_and_beta_true_are_forwarded() {
    let h = harness().await;
    let response = h
        .client
        .post(format!("{}/anthropic/v1/messages?beta=true&evil=1", h.url))
        .header("x-api-key", TASK_TOKEN)
        .header(
            "anthropic-beta",
            "files-api-2025-04-14,interleaved-thinking-2025-05-14",
        )
        .body(MESSAGE)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let only_unlisted = h
        .client
        .post(format!("{}/anthropic/v1/messages?other=1", h.url))
        .header("x-api-key", TASK_TOKEN)
        .header("anthropic-beta", "files-api-2025-04-14")
        .body(MESSAGE)
        .send()
        .await
        .unwrap();
    assert_eq!(only_unlisted.status(), 200);

    let seen = Harness::received(&h.anthropic);
    assert_eq!(seen[0].uri, "/v1/messages?beta=true");
    assert_eq!(
        seen[0].headers["anthropic-beta"],
        "interleaved-thinking-2025-05-14"
    );
    assert_eq!(seen[1].uri, "/v1/messages");
    assert!(seen[1].headers.get("anthropic-beta").is_none());
}

#[tokio::test]
async fn s1_a_body_over_10_mib_is_413_and_never_forwarded() {
    let h = harness().await;
    let big = format!(
        r#"{{"model":"{MODEL}","max_tokens":1,"pad":"{}"}}"#,
        "x".repeat(10 * 1024 * 1024)
    );
    let declared = h
        .client
        .post(format!("{}/anthropic/v1/messages", h.url))
        .header("x-api-key", TASK_TOKEN)
        .body(big.clone())
        .send()
        .await;
    if let Ok(declared) = declared {
        assert_eq!(declared.status(), 413);
    }
    let chunks: Vec<Result<Vec<u8>, Infallible>> = big
        .as_bytes()
        .chunks(64 * 1024)
        .map(|chunk| Ok(chunk.to_vec()))
        .collect();
    let streamed = h
        .client
        .post(format!("{}/anthropic/v1/messages", h.url))
        .header("x-api-key", TASK_TOKEN)
        .body(reqwest::Body::wrap_stream(stream::iter(chunks)))
        .send()
        .await;
    if let Ok(streamed) = streamed {
        assert_eq!(streamed.status(), 413);
    }
    assert!(Harness::received(&h.anthropic).is_empty());
}

#[tokio::test]
async fn s8_unknown_tokens_are_remembered_for_a_moment() {
    let h = harness().await;
    for _ in 0..4 {
        assert_eq!(h.messages("/v1/messages", "nope").await.status(), 401);
    }
    assert_eq!(h.lookups.load(Ordering::SeqCst), 1);
    assert_eq!(h.messages("/v1/messages", TASK_TOKEN).await.status(), 200);
}

#[tokio::test]
async fn s8_a_grant_serves_at_most_16_requests_at_once() {
    let upstream = Router::new().fallback(|| async {
        tokio::time::sleep(Duration::from_millis(800)).await;
        "slow"
    });
    let anthropic_url = serve(upstream).await;
    let (github_url, github) = capturing_upstream().await;
    let h = Arc::new(
        start(
            Fake::with(grant()),
            anthropic_url,
            github_url,
            Log::default(),
            github,
        )
        .await,
    );
    let calls: Vec<_> = (0..20)
        .map(|_| {
            let h = Arc::clone(&h);
            tokio::spawn(async move { h.messages("/v1/messages", TASK_TOKEN).await.status() })
        })
        .collect();
    let mut statuses = Vec::new();
    for call in calls {
        statuses.push(call.await.unwrap().as_u16());
    }
    assert_eq!(statuses.iter().filter(|s| **s == 200).count(), 16);
    assert_eq!(statuses.iter().filter(|s| **s == 429).count(), 4);
    assert_eq!(h.messages("/v1/messages", TASK_TOKEN).await.status(), 200);
}

fn codex_body() -> serde_json::Value {
    serde_json::json!({
        "model": CODEX_MODEL,
        "stream": true,
        "store": false,
        "input": [{"role": "user", "content": "hi"}],
        "tools": [
            {"type": "function", "name": "shell", "parameters": {}},
            {"type": "custom", "name": "apply_patch"},
        ],
        "tool_choice": "auto",
        "parallel_tool_calls": true,
        "reasoning": {"effort": "high"},
        "include": ["reasoning.encrypted_content"],
        "prompt_cache_key": "k",
        "text": {},
        "client_metadata": {},
    })
}

#[tokio::test]
async fn a_codex_request_is_forwarded_with_the_organizations_openai_key() {
    let sse = "event: response.created\ndata: {}\n\nevent: response.completed\ndata: {}\n\n";
    let upstream = Router::new().fallback(move || async move {
        Response::builder()
            .header("content-type", "text/event-stream")
            .body(Body::from(sse))
            .unwrap()
    });
    let openai_url = serve(upstream).await;
    let (log_url, log) = capturing_upstream().await;
    let capture_h = start_openai(Fake::with(codex_grant()), log_url, log).await;
    let response = capture_h.responses(TASK_TOKEN, codex_body()).await;
    assert_eq!(response.status(), 200);
    let seen = Harness::received(&capture_h.openai);
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.method, Method::POST);
    assert_eq!(request.uri, "/v1/responses");
    assert_eq!(
        request.headers["authorization"],
        format!("Bearer {OPENAI_KEY}")
    );
    assert_eq!(request.headers["content-type"], "application/json");
    assert_eq!(request.headers["accept"], "text/event-stream");
    assert!(request.headers.get("openai-organization").is_none());
    assert!(request.headers.get("openai-project").is_none());
    assert!(request.headers.get("x-api-key").is_none());
    for (_, value) in &request.headers {
        assert!(!value.to_str().unwrap_or_default().contains(TASK_TOKEN));
    }
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        codex_body()
    );
    assert!(Harness::received(&capture_h.anthropic).is_empty());

    let streaming = start_openai(Fake::with(codex_grant()), openai_url, Log::default()).await;
    let response = streaming.responses(TASK_TOKEN, codex_body()).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.text().await.unwrap(), sse);
}

#[tokio::test]
async fn the_openai_query_string_is_dropped() {
    let h = harness_with(Fake::with(codex_grant())).await;
    let response = h
        .client
        .post(format!("{}/openai/v1/responses?api-version=x&a=b", h.url))
        .header("authorization", format!("Bearer {TASK_TOKEN}"))
        .header("content-type", "application/json")
        .body(codex_body().to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(Harness::received(&h.openai)[0].uri, "/v1/responses");
}

#[tokio::test]
async fn a_codex_grant_cannot_use_the_anthropic_route() {
    let h = harness_with(Fake::with(codex_grant())).await;
    let response = h
        .messages_with(serde_json::json!({"model": CODEX_MODEL}))
        .await;
    assert_eq!(response.status(), 403);
    assert!(Harness::received(&h.anthropic).is_empty());
    assert!(Harness::received(&h.openai).is_empty());
}

#[tokio::test]
async fn a_claude_grant_cannot_use_the_openai_route() {
    let h = harness().await;
    let response = h
        .responses(TASK_TOKEN, serde_json::json!({"model": MODEL}))
        .await;
    assert_eq!(response.status(), 403);
    assert!(Harness::received(&h.openai).is_empty());
}

#[tokio::test]
async fn openai_bodies_that_escape_the_task_are_refused() {
    let h = harness_with(Fake::with(codex_grant())).await;
    let with = |field: &str, value: serde_json::Value| {
        let mut body = codex_body();
        body[field] = value;
        body
    };
    let refused = [
        serde_json::json!([1]),
        with("model", "gpt-5".into()),
        with("store", true.into()),
        with("previous_response_id", "resp_1".into()),
        with("conversation", "conv_1".into()),
        with("prompt", serde_json::json!({"id": "p"})),
        with("background", true.into()),
        with("max_output_tokens", 128_001.into()),
        with("tools", serde_json::json!([{"type": "web_search"}])),
        with("tools", serde_json::json!([{"name": "untyped"}])),
        with("tools", serde_json::json!([{"type": "image_generation"}])),
    ];
    for body in refused {
        let response = h.responses(TASK_TOKEN, body.clone()).await;
        assert_eq!(response.status(), 400, "{body}");
    }
    let not_json = h
        .client
        .post(format!("{}/openai/v1/responses", h.url))
        .header("authorization", format!("Bearer {TASK_TOKEN}"))
        .body("nope")
        .send()
        .await
        .unwrap();
    assert_eq!(not_json.status(), 400);
    assert!(Harness::received(&h.openai).is_empty());
}

#[tokio::test]
async fn only_post_v1_responses_is_served_on_the_openai_route() {
    let h = harness_with(Fake::with(codex_grant())).await;
    let models = h
        .client
        .get(format!("{}/openai/v1/models", h.url))
        .header("authorization", format!("Bearer {TASK_TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(models.status(), 403);
    let other = h
        .client
        .post(format!("{}/openai/v1/chat/completions", h.url))
        .header("authorization", format!("Bearer {TASK_TOKEN}"))
        .body(codex_body().to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(other.status(), 403);
    let get = h
        .client
        .get(format!("{}/openai/v1/responses", h.url))
        .header("authorization", format!("Bearer {TASK_TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 403);
    assert!(Harness::received(&h.openai).is_empty());
}

#[tokio::test]
async fn an_unknown_token_is_401_on_the_openai_route() {
    let h = harness_with(Fake::with(codex_grant())).await;
    let response = h.responses("who-knows", codex_body()).await;
    assert_eq!(response.status(), 401);
    let none = h
        .client
        .post(format!("{}/openai/v1/responses", h.url))
        .body(codex_body().to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(none.status(), 401);
    assert!(Harness::received(&h.openai).is_empty());
}

#[tokio::test]
async fn an_organization_without_an_openai_key_gets_a_clear_403() {
    let mut fake = Fake::with(codex_grant());
    fake.openai_key = None;
    let h = harness_with(fake).await;
    let response = h.responses(TASK_TOKEN, codex_body()).await;
    assert_eq!(response.status(), 403);
    assert!(response.text().await.unwrap().contains("OpenAI key"));
    assert!(Harness::received(&h.openai).is_empty());
}

fn stream_path(model: &str) -> String {
    format!("/v1beta/models/{model}:streamGenerateContent?alt=sse")
}

fn agy_json() -> serde_json::Value {
    serde_json::from_str(AGY_BODY).unwrap()
}

#[tokio::test]
async fn an_agy_request_is_forwarded_with_the_organizations_google_key() {
    let h = harness_with(Fake::with(agy_grant())).await;
    let response = h.agy(AGY_BODY).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "upstream-ok");

    let seen = Harness::received(&h.google);
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.method, Method::POST);
    assert_eq!(
        request.uri,
        "/v1beta/models/gemini-3.1-pro-preview:streamGenerateContent?alt=sse"
    );
    assert_eq!(request.headers["x-goog-api-key"], GOOGLE_KEY);
    assert_eq!(request.headers["content-type"], "application/json");
    assert!(request.headers.get("authorization").is_none());
    assert!(request.headers.get("x-goog-api-client").is_none());
    assert!(request.headers.get("x-api-key").is_none());
    assert!(request.headers.get("user-agent").is_none());
    assert!(
        !request
            .headers
            .get("accept-encoding")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .contains("x-agy-coding")
    );
    for (_, value) in &request.headers {
        assert!(!value.to_str().unwrap_or_default().contains(TASK_TOKEN));
    }
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        serde_json::from_str::<serde_json::Value>(AGY_BODY).unwrap()
    );
    assert!(Harness::received(&h.anthropic).is_empty());
    assert!(Harness::received(&h.openai).is_empty());
}

async fn start_google(fake: Fake, google_url: String, google: Log) -> Harness {
    let (anthropic_url, anthropic) = capturing_upstream().await;
    let (github_url, github) = capturing_upstream().await;
    let (openai_url, _) = capturing_upstream().await;
    start_with(
        fake,
        anthropic_url,
        openai_url,
        github_url,
        anthropic,
        github,
        (google_url, google),
    )
    .await
}

#[tokio::test]
async fn the_google_stream_is_relayed() {
    let sse = "data: {\"candidates\":[]}\n\ndata: {\"usageMetadata\":{}}\n\n";
    let upstream = Router::new().fallback(move || async move {
        Response::builder()
            .header("content-type", "text/event-stream")
            .body(Body::from(sse))
            .unwrap()
    });
    let h = start_google(
        Fake::with(agy_grant()),
        serve(upstream).await,
        Log::default(),
    )
    .await;
    let response = h.agy(AGY_BODY).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.text().await.unwrap(), sse);
}

#[tokio::test]
async fn the_grant_model_is_translated_to_the_wire_model_and_the_cli_id_is_refused() {
    let h = harness_with(Fake::with(agy_grant())).await;
    let cli_id = h
        .generate(
            &stream_path(AGY_MODEL),
            ("x-goog-api-key", TASK_TOKEN),
            AGY_BODY,
        )
        .await;
    assert_eq!(cli_id.status(), 403);
    assert!(Harness::received(&h.google).is_empty());
    assert_eq!(h.agy(AGY_BODY).await.status(), 200);
    assert!(
        Harness::received(&h.google)[0]
            .uri
            .starts_with("/v1beta/models/gemini-3.1-pro-preview:")
    );
}

#[tokio::test]
async fn helper_models_are_refused_without_an_upstream_call() {
    let h = harness_with(Fake::with(agy_grant())).await;
    for model in [
        HELPER_WIRE,
        "gemini-3.8-flash",
        "gemini-3.1-pro-preview-x",
        "",
    ] {
        let response = h
            .generate(
                &stream_path(model),
                ("x-goog-api-key", TASK_TOKEN),
                AGY_BODY,
            )
            .await;
        assert_eq!(response.status(), 403, "{model}");
    }
    assert!(Harness::received(&h.google).is_empty());
}

#[tokio::test]
async fn other_google_paths_and_methods_are_refused() {
    let h = harness_with(Fake::with(agy_grant())).await;
    for path in [
        format!("/v1beta/models/{AGY_WIRE}:countTokens"),
        format!("/v1beta/models/{AGY_WIRE}:embedContent"),
        format!("/v1beta/models/{AGY_WIRE}"),
        "/v1beta/models".to_owned(),
        "/v1beta/files".to_owned(),
        format!("/v1beta/models/{AGY_WIRE}:generateContent/extra"),
        format!("/v1beta/models/{AGY_WIRE}%3AgenerateContent"),
        format!("/v1/models/{AGY_WIRE}:generateContent"),
        format!("/v1beta/tunedModels/{AGY_WIRE}:generateContent"),
    ] {
        let response = h
            .generate(&path, ("x-goog-api-key", TASK_TOKEN), AGY_BODY)
            .await;
        assert_eq!(response.status(), 403, "{path}");
    }
    for path in [
        format!("/v1beta/models/{AGY_WIRE}:streamGenerateContent?alt=sse"),
        "/v1beta/models".to_owned(),
    ] {
        let get = h
            .client
            .get(format!("{}/google{path}", h.url))
            .header("x-goog-api-key", TASK_TOKEN)
            .send()
            .await
            .unwrap();
        assert_eq!(get.status(), 403, "{path}");
    }
    assert!(Harness::received(&h.google).is_empty());
}

#[tokio::test]
async fn the_google_query_keeps_only_alt_sse() {
    let h = harness_with(Fake::with(agy_grant())).await;
    let path = format!("/v1beta/models/{AGY_WIRE}:generateContent");
    let with_extras = h
        .generate(
            &format!("{path}?alt=sse&key=stolen&cachedContent=x"),
            ("x-goog-api-key", TASK_TOKEN),
            AGY_BODY,
        )
        .await;
    assert_eq!(with_extras.status(), 200);
    let without = h
        .generate(
            &format!("{path}?key=stolen&alt=json"),
            ("x-goog-api-key", TASK_TOKEN),
            AGY_BODY,
        )
        .await;
    assert_eq!(without.status(), 200);
    let seen = Harness::received(&h.google);
    assert_eq!(
        seen[0].uri,
        "/v1beta/models/gemini-3.1-pro-preview:generateContent?alt=sse"
    );
    assert_eq!(
        seen[1].uri,
        "/v1beta/models/gemini-3.1-pro-preview:generateContent"
    );
}

#[tokio::test]
async fn other_harness_grants_cannot_use_the_google_route() {
    for grant in [grant(), codex_grant()] {
        let h = harness_with(Fake::with(grant)).await;
        assert_eq!(h.agy(AGY_BODY).await.status(), 403);
        assert!(Harness::received(&h.google).is_empty());
    }
}

#[tokio::test]
async fn an_antigravity_grant_cannot_use_the_openai_or_anthropic_routes() {
    let h = harness_with(Fake::with(agy_grant())).await;
    let responses = h
        .responses(TASK_TOKEN, serde_json::json!({"model": AGY_MODEL}))
        .await;
    assert_eq!(responses.status(), 403);
    let messages = h
        .messages_with(serde_json::json!({"model": AGY_MODEL}))
        .await;
    assert_eq!(messages.status(), 403);
    assert!(Harness::received(&h.openai).is_empty());
    assert!(Harness::received(&h.anthropic).is_empty());
}

#[tokio::test]
async fn google_bodies_that_escape_the_task_are_refused() {
    let h = harness_with(Fake::with(agy_grant())).await;
    let with = |field: &str, value: serde_json::Value| {
        let mut body = agy_json();
        body[field] = value;
        body.to_string()
    };
    let remote_part = serde_json::json!([{"role": "user", "parts": [
        {"fileData": {"fileUri": "https://attacker.example/", "mimeType": "image/png"}},
    ]}]);
    let snake_part = serde_json::json!([{"role": "user", "parts": [
        {"file_data": {"file_uri": "gs://bucket/object"}},
    ]}]);
    let refused = [
        "[1]".to_owned(),
        "nope".to_owned(),
        with("cachedContent", "cachedContents/x".into()),
        with("tools", serde_json::json!([{"googleSearch": {}}])),
        with("tools", serde_json::json!([{"codeExecution": {}}])),
        with("tools", serde_json::json!([{"urlContext": {}}])),
        with(
            "tools",
            serde_json::json!([{"functionDeclarations": [], "googleSearch": {}}]),
        ),
        with(
            "generationConfig",
            serde_json::json!({"maxOutputTokens": 128_001}),
        ),
        with("contents", remote_part),
        with("contents", snake_part.clone()),
        with(
            "systemInstruction",
            serde_json::json!({"parts": snake_part[0]["parts"]}),
        ),
    ];
    for body in refused {
        let response = h.agy(&body).await;
        assert_eq!(response.status(), 400, "{body}");
    }
    assert!(Harness::received(&h.google).is_empty());
    let inline = with(
        "contents",
        serde_json::json!([{"role": "user", "parts": [
            {"inlineData": {"mimeType": "image/png", "data": "AAAA"}},
        ]}]),
    );
    assert_eq!(h.agy(&inline).await.status(), 200);
}

#[tokio::test]
async fn an_unknown_token_is_401_on_the_google_route() {
    let h = harness_with(Fake::with(agy_grant())).await;
    let response = h
        .generate(
            &stream_path(AGY_WIRE),
            ("x-goog-api-key", "who-knows"),
            AGY_BODY,
        )
        .await;
    assert_eq!(response.status(), 401);
    let none = h
        .client
        .post(format!("{}/google{}", h.url, stream_path(AGY_WIRE)))
        .body(AGY_BODY)
        .send()
        .await
        .unwrap();
    assert_eq!(none.status(), 401);
    assert!(Harness::received(&h.google).is_empty());
}

#[tokio::test]
async fn an_organization_without_a_google_key_gets_a_clear_403() {
    let mut fake = Fake::with(agy_grant());
    fake.google_key = None;
    let h = harness_with(fake).await;
    let response = h.agy(AGY_BODY).await;
    assert_eq!(response.status(), 403);
    assert!(response.text().await.unwrap().contains("Google key"));
    assert!(Harness::received(&h.google).is_empty());
}
