use std::{collections::HashMap, fmt};

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    execution::{DraftPullRequest, PullRequestUrl, RepositoryRef, ports::CodeHost},
};
use chrono::{DateTime, Duration, Utc};
use reqwest::{
    RequestBuilder, Response,
    header::{ACCEPT, AUTHORIZATION, LINK, USER_AGENT},
};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

use crate::{GithubError, jwt};

const DEFAULT_API_BASE: &str = "https://api.github.com";
const DEFAULT_OAUTH_BASE: &str = "https://github.com";
const API_VERSION: &str = "2022-11-28";
const AGENT: &str = concat!("capitainerie/", env!("CARGO_PKG_VERSION"));
const REFRESH_MARGIN_MINUTES: i64 = 5;

#[derive(Clone)]
pub struct GithubApp {
    pub app_id: u64,
    pub private_key_pem: SecretToken,
    pub api_base: String,
    pub oauth_base: String,
    pub client_id: Option<String>,
    pub client_secret: Option<SecretToken>,
}

impl GithubApp {
    pub fn new(app_id: u64, private_key_pem: SecretToken) -> Self {
        Self {
            app_id,
            private_key_pem,
            api_base: DEFAULT_API_BASE.to_owned(),
            oauth_base: DEFAULT_OAUTH_BASE.to_owned(),
            client_id: None,
            client_secret: None,
        }
    }
}

impl fmt::Debug for GithubApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GithubApp")
            .field("app_id", &self.app_id)
            .field("api_base", &self.api_base)
            .field("oauth_base", &self.oauth_base)
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

/// A user-to-server token: used once to check an installation, never stored.
pub struct UserToken(SecretToken);

impl fmt::Debug for UserToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UserToken").field(&"***").finish()
    }
}

#[derive(Clone)]
pub struct InstallationToken {
    pub token: SecretToken,
    pub expires_at: DateTime<Utc>,
}

impl fmt::Debug for InstallationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstallationToken")
            .field("token", &"***")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl InstallationToken {
    fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        now < self.expires_at - Duration::minutes(REFRESH_MARGIN_MINUTES)
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct OAuthResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct UserInstallationsPage {
    installations: Vec<UserInstallationEntry>,
}

#[derive(Deserialize)]
struct UserInstallationEntry {
    id: i64,
}

#[derive(Deserialize)]
struct RepositoriesPage {
    repositories: Vec<RepositoryEntry>,
}

#[derive(Deserialize)]
struct RepositoryEntry {
    full_name: String,
}

#[derive(Deserialize)]
struct RepositoryDetails {
    default_branch: String,
}

#[derive(Deserialize)]
struct InstallationDetails {
    account: AccountEntry,
}

#[derive(Deserialize)]
struct AccountEntry {
    login: String,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Deserialize)]
struct Membership {
    role: String,
    state: String,
}

#[derive(Deserialize)]
struct AuthenticatedUser {
    login: String,
}

#[derive(Deserialize)]
struct RefDetails {
    object: RefObject,
}

#[derive(Deserialize)]
struct RefObject {
    sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountKind {
    User,
    Organization,
    Other,
}

/// The account an installation belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationAccount {
    pub login: String,
    pub kind: AccountKind,
}

#[derive(Deserialize)]
struct PullRequestResponse {
    html_url: String,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: String,
    #[serde(default)]
    errors: Vec<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

#[derive(Deserialize)]
struct PullRequestListEntry {
    html_url: String,
}

const PULL_REQUEST_EXISTS: &str = "A pull request already exists";

type CacheKey = (i64, Option<RepositoryRef>);

pub struct GithubClient {
    app: GithubApp,
    http: reqwest::Client,
    cache: Mutex<HashMap<CacheKey, InstallationToken>>,
}

impl GithubClient {
    pub fn new(app: GithubApp) -> Result<Self, GithubError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(GithubError::transport)?;
        Ok(Self::with_http(app, http))
    }

    pub fn with_http(app: GithubApp, http: reqwest::Client) -> Self {
        Self {
            app,
            http,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.app.api_base.trim_end_matches('/'))
    }

    fn request(&self, builder: RequestBuilder) -> RequestBuilder {
        builder
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, AGENT)
    }

    async fn send(builder: RequestBuilder) -> Result<Response, GithubError> {
        let response = builder.send().await.map_err(GithubError::transport)?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let message = response
            .json::<ErrorBody>()
            .await
            .map(|body| {
                let details: Vec<_> = body
                    .errors
                    .into_iter()
                    .filter_map(|detail| detail.message)
                    .collect();
                if details.is_empty() {
                    body.message
                } else {
                    format!("{}: {}", body.message, details.join("; "))
                }
            })
            .unwrap_or_else(|_| {
                status
                    .canonical_reason()
                    .unwrap_or("unknown error")
                    .to_owned()
            });
        Err(GithubError::Status {
            status: status.as_u16(),
            message,
        })
    }

    async fn decode<T: for<'de> Deserialize<'de>>(response: Response) -> Result<T, GithubError> {
        response
            .json()
            .await
            .map_err(|error| GithubError::Decode(error.without_url().to_string()))
    }

    /// `repository` of `None` asks for a token over every repository the
    /// installation can see, which listing repositories requires.
    async fn token_for(
        &self,
        installation_id: i64,
        repository: Option<&RepositoryRef>,
    ) -> Result<InstallationToken, GithubError> {
        let key = (installation_id, repository.cloned());
        if let Some(token) = self.cache.lock().await.get(&key)
            && token.is_fresh(Utc::now())
        {
            return Ok(token.clone());
        }

        let body = match repository {
            Some(repository) => json!({
                "repositories": [repository.name()],
                "permissions": {"contents": "write", "pull_requests": "write"},
            }),
            None => json!({"permissions": {"metadata": "read"}}),
        };
        let jwt = jwt::sign(
            self.app.app_id,
            self.app.private_key_pem.expose(),
            Utc::now(),
        )?;
        let response = Self::send(
            self.request(
                self.http
                    .post(self.url(&format!(
                        "/app/installations/{installation_id}/access_tokens"
                    )))
                    .bearer_auth(jwt)
                    .json(&body),
            ),
        )
        .await?;
        let issued: TokenResponse = Self::decode(response).await?;
        let token = InstallationToken {
            token: SecretToken::new(issued.token),
            expires_at: issued.expires_at,
        };
        self.cache.lock().await.insert(key, token.clone());
        Ok(token)
    }

    pub async fn installation_token(
        &self,
        installation_id: i64,
        repository: &RepositoryRef,
    ) -> Result<InstallationToken, GithubError> {
        self.token_for(installation_id, Some(repository)).await
    }

    pub async fn default_branch(
        &self,
        installation_id: i64,
        repository: &RepositoryRef,
    ) -> Result<String, GithubError> {
        let token = self.token_for(installation_id, Some(repository)).await?;
        let response = Self::send(
            self.request(
                self.http
                    .get(self.url(&format!(
                        "/repos/{}/{}",
                        repository.owner(),
                        repository.name()
                    )))
                    .header(AUTHORIZATION, format!("Bearer {}", token.token.expose())),
            ),
        )
        .await?;
        Ok(Self::decode::<RepositoryDetails>(response)
            .await?
            .default_branch)
    }

    /// The login of the account the App is installed on; authenticated as the
    /// App itself, since the installation id comes from an untrusted redirect.
    pub async fn installation_account(
        &self,
        installation_id: i64,
    ) -> Result<InstallationAccount, GithubError> {
        let jwt = jwt::sign(
            self.app.app_id,
            self.app.private_key_pem.expose(),
            Utc::now(),
        )?;
        let response = Self::send(
            self.request(
                self.http
                    .get(self.url(&format!("/app/installations/{installation_id}")))
                    .bearer_auth(jwt),
            ),
        )
        .await?;
        let account = Self::decode::<InstallationDetails>(response).await?.account;
        Ok(InstallationAccount {
            kind: match account.kind.as_str() {
                "User" => AccountKind::User,
                "Organization" => AccountKind::Organization,
                _ => AccountKind::Other,
            },
            login: account.login,
        })
    }

    /// Whether the connecting user may bind this installation: an admin of the
    /// organization it is installed on, or the user it is installed on.
    pub async fn user_owns_installation(
        &self,
        token: &UserToken,
        account: &InstallationAccount,
    ) -> Result<bool, GithubError> {
        let authorization = format!("Bearer {}", token.0.expose());
        match account.kind {
            AccountKind::Organization => {
                let response = Self::send(
                    self.request(
                        self.http
                            .get(self.url(&format!("/user/memberships/orgs/{}", account.login)))
                            .header(AUTHORIZATION, authorization),
                    ),
                )
                .await;
                match response {
                    Ok(response) => {
                        let membership: Membership = Self::decode(response).await?;
                        Ok(membership.role == "admin" && membership.state == "active")
                    }
                    Err(GithubError::Status {
                        status: 403 | 404, ..
                    }) => Ok(false),
                    Err(error) => Err(error),
                }
            }
            AccountKind::User => {
                let response = Self::send(
                    self.request(
                        self.http
                            .get(self.url("/user"))
                            .header(AUTHORIZATION, authorization),
                    ),
                )
                .await?;
                let user: AuthenticatedUser = Self::decode(response).await?;
                Ok(user.login.eq_ignore_ascii_case(&account.login))
            }
            AccountKind::Other => Ok(false),
        }
    }

    async fn branch_sha(
        &self,
        installation_id: i64,
        repository: &RepositoryRef,
        branch: &str,
    ) -> Result<Option<String>, GithubError> {
        let token = self.token_for(installation_id, Some(repository)).await?;
        let response = Self::send(
            self.request(
                self.http
                    .get(self.url(&format!(
                        "/repos/{}/{}/git/ref/heads/{branch}",
                        repository.owner(),
                        repository.name()
                    )))
                    .header(AUTHORIZATION, format!("Bearer {}", token.token.expose())),
            ),
        )
        .await;
        match response {
            Ok(response) => Ok(Some(Self::decode::<RefDetails>(response).await?.object.sha)),
            Err(GithubError::Status { status: 404, .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn oauth_configured(&self) -> bool {
        self.app.client_id.is_some() && self.app.client_secret.is_some()
    }

    /// Exchanges the `code` GitHub appends to the setup redirect when the App
    /// asks for user authorization during installation.
    pub async fn exchange_user_code(&self, code: &str) -> Result<UserToken, GithubError> {
        let (Some(client_id), Some(client_secret)) = (&self.app.client_id, &self.app.client_secret)
        else {
            return Err(GithubError::OAuthNotConfigured);
        };
        let url = format!(
            "{}/login/oauth/access_token",
            self.app.oauth_base.trim_end_matches('/')
        );
        let response = Self::send(
            self.http
                .post(url)
                .header(ACCEPT, "application/json")
                .header(USER_AGENT, AGENT)
                .json(&json!({
                    "client_id": client_id,
                    "client_secret": client_secret.expose(),
                    "code": code,
                })),
        )
        .await?;
        let exchanged: OAuthResponse = Self::decode(response).await?;
        match (exchanged.access_token, exchanged.error) {
            (Some(token), _) => Ok(UserToken(SecretToken::new(token))),
            (None, error) => Err(GithubError::Status {
                status: 400,
                message: error.unwrap_or_else(|| "no access token returned".to_owned()),
            }),
        }
    }

    pub async fn user_can_access_installation(
        &self,
        token: &UserToken,
        installation_id: i64,
    ) -> Result<bool, GithubError> {
        let api_base = self.app.api_base.trim_end_matches('/');
        let mut next = Some(self.url("/user/installations?per_page=100"));
        while let Some(url) = next {
            let response = Self::send(
                self.request(
                    self.http
                        .get(&url)
                        .header(AUTHORIZATION, format!("Bearer {}", token.0.expose())),
                ),
            )
            .await?;
            next = response
                .headers()
                .get(LINK)
                .and_then(|value| value.to_str().ok())
                .and_then(next_link)
                .filter(|link| same_origin(link, api_base))
                .map(str::to_owned);
            let page: UserInstallationsPage = Self::decode(response).await?;
            if page
                .installations
                .iter()
                .any(|entry| entry.id == installation_id)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn list_repositories(
        &self,
        installation_id: i64,
    ) -> Result<Vec<RepositoryRef>, GithubError> {
        let token = self.token_for(installation_id, None).await?;
        let api_base = self.app.api_base.trim_end_matches('/');
        let mut next = Some(self.url("/installation/repositories?per_page=100"));
        let mut repositories = Vec::new();
        while let Some(url) = next {
            let response = Self::send(
                self.request(
                    self.http
                        .get(&url)
                        .header(AUTHORIZATION, format!("Bearer {}", token.token.expose())),
                ),
            )
            .await?;
            next = response
                .headers()
                .get(LINK)
                .and_then(|value| value.to_str().ok())
                .and_then(next_link)
                .filter(|link| same_origin(link, api_base))
                .map(str::to_owned);
            let page: RepositoriesPage = Self::decode(response).await?;
            for entry in page.repositories {
                match entry.full_name.parse() {
                    Ok(repository) => repositories.push(repository),
                    Err(_) => tracing::warn!(
                        full_name = %entry.full_name,
                        "skipping a repository with an unsupported name"
                    ),
                }
            }
        }
        Ok(repositories)
    }

    async fn create_draft_pr(
        &self,
        installation_id: i64,
        pull_request: &DraftPullRequest,
    ) -> Result<PullRequestUrl, GithubError> {
        let repository = &pull_request.repository;
        let token = self.token_for(installation_id, Some(repository)).await?;
        let pulls_url = self.url(&format!(
            "/repos/{}/{}/pulls",
            repository.owner(),
            repository.name()
        ));
        let created = Self::send(
            self.request(
                self.http
                    .post(&pulls_url)
                    .header(AUTHORIZATION, format!("Bearer {}", token.token.expose()))
                    .json(&json!({
                        "title": pull_request.title,
                        "head": pull_request.head,
                        "base": pull_request.base,
                        "body": pull_request.body,
                        "draft": true,
                    })),
            ),
        )
        .await;
        match created {
            Ok(response) => {
                let created: PullRequestResponse = Self::decode(response).await?;
                Ok(PullRequestUrl(created.html_url))
            }
            Err(GithubError::Status {
                status: 422,
                message,
            }) if message.contains(PULL_REQUEST_EXISTS) => {
                match self
                    .open_pull_request_of(&pulls_url, &token, repository, &pull_request.head)
                    .await?
                {
                    Some(url) => Ok(url),
                    None => Err(GithubError::Status {
                        status: 422,
                        message,
                    }),
                }
            }
            Err(error) => Err(error),
        }
    }

    async fn open_pull_request_of(
        &self,
        pulls_url: &str,
        token: &InstallationToken,
        repository: &RepositoryRef,
        branch: &str,
    ) -> Result<Option<PullRequestUrl>, GithubError> {
        let response = Self::send(
            self.request(
                self.http
                    .get(pulls_url)
                    .query(&[
                        ("head", format!("{}:{branch}", repository.owner())),
                        ("state", "open".to_owned()),
                    ])
                    .header(AUTHORIZATION, format!("Bearer {}", token.token.expose())),
            ),
        )
        .await?;
        let open: Vec<PullRequestListEntry> = Self::decode(response).await?;
        Ok(open
            .into_iter()
            .next()
            .map(|entry| PullRequestUrl(entry.html_url)))
    }
}

fn origin(url: &str) -> Option<(String, String, u16)> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.contains('@') || authority.is_empty() {
        return None;
    }
    let default_port = match scheme {
        "https" => 443,
        "http" => 80,
        _ => return None,
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.ends_with(':') => (host, port.parse().ok()?),
        _ => (authority, default_port),
    };
    Some((scheme.to_ascii_lowercase(), host.to_ascii_lowercase(), port))
}

/// Same scheme, host and port: a `Link` header must not lead the client, and
/// the token it carries, to another server.
fn same_origin(link: &str, base: &str) -> bool {
    match (origin(link), origin(base)) {
        (Some(link), Some(base)) => link == base,
        _ => false,
    }
}

pub(crate) fn next_link(header: &str) -> Option<&str> {
    header.split(',').find_map(|part| {
        let (target, params) = part.split_once(';')?;
        let is_next = params
            .split(';')
            .any(|param| param.trim().replace(' ', "") == "rel=\"next\"");
        let url = target.trim().strip_prefix('<')?.strip_suffix('>')?;
        is_next.then_some(url)
    })
}

impl CodeHost for GithubClient {
    async fn repositories(&self, installation_id: i64) -> Result<Vec<RepositoryRef>, CoreError> {
        Ok(self.list_repositories(installation_id).await?)
    }

    async fn branch_head(
        &self,
        installation_id: i64,
        repository: &RepositoryRef,
        branch: &str,
    ) -> Result<Option<String>, CoreError> {
        Ok(self.branch_sha(installation_id, repository, branch).await?)
    }

    async fn open_draft_pr(
        &self,
        installation_id: i64,
        pull_request: DraftPullRequest,
    ) -> Result<PullRequestUrl, CoreError> {
        Ok(self.create_draft_pr(installation_id, &pull_request).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex as StdMutex};

    use axum::{
        Json, Router,
        extract::{Query, State},
        http::{HeaderMap, StatusCode as AxumStatus},
        response::IntoResponse,
        routing::{get, post},
    };
    use serde_json::Value;

    use super::*;

    const PRIVATE: &str = include_str!("../tests/fixtures/test-only-app-key.pem");

    #[derive(Default)]
    struct Recorded {
        token_bodies: Vec<Value>,
        token_headers: Vec<HeaderMap>,
        pull_bodies: Vec<Value>,
        expires_at: Option<String>,
        fail_pulls_with: Option<u16>,
        pull_exists: bool,
        listed_pulls: Vec<Value>,
        account_kind: Option<String>,
        membership: Option<Value>,
        user_login: Option<String>,
        branch_sha: Option<String>,
        list_queries: Vec<HashMap<String, String>>,
        base: String,
        oauth_bodies: Vec<Value>,
        oauth_reply: Option<Value>,
        user_auth: Vec<String>,
    }

    type Shared = Arc<StdMutex<Recorded>>;

    async fn issue_token(
        State(shared): State<Shared>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        let mut recorded = shared.lock().unwrap();
        recorded.token_bodies.push(body);
        recorded.token_headers.push(headers);
        let n = recorded.token_bodies.len();
        let expires_at = recorded
            .expires_at
            .clone()
            .unwrap_or_else(|| (Utc::now() + Duration::hours(1)).to_rfc3339());
        Json(json!({"token": format!("ghs_secret{n}"), "expires_at": expires_at}))
    }

    async fn repositories(
        State(shared): State<Shared>,
        Query(query): Query<HashMap<String, String>>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        assert_eq!(headers["authorization"], "Bearer ghs_secret1");
        assert_eq!(query["per_page"], "100");
        let base = shared.lock().unwrap().base.clone();
        match query.get("page").map(String::as_str) {
            None => (
                [(
                    "link",
                    format!("<{base}/installation/repositories?per_page=100&page=2>; rel=\"next\""),
                )],
                Json(
                    json!({"repositories": [{"full_name": "acme/api"}, {"full_name": "acme/web"}]}),
                ),
            )
                .into_response(),
            _ => Json(json!({"repositories": [{"full_name": "acme/docs"}]})).into_response(),
        }
    }

    async fn pulls(
        State(shared): State<Shared>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> impl IntoResponse {
        let mut recorded = shared.lock().unwrap();
        if recorded.pull_exists {
            return (
                AxumStatus::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "message": "Validation Failed",
                    "errors": [{"resource": "PullRequest", "code": "custom", "message": "A pull request already exists for acme:capitainerie/run-1."}],
                })),
            )
                .into_response();
        }
        if let Some(status) = recorded.fail_pulls_with {
            return (
                AxumStatus::from_u16(status).unwrap(),
                Json(json!({"message": "Validation Failed"})),
            )
                .into_response();
        }
        assert_eq!(headers["authorization"], "Bearer ghs_secret1");
        recorded.pull_bodies.push(body);
        Json(json!({"html_url": "https://github.com/acme/api/pull/9"})).into_response()
    }

    async fn list_pulls(
        State(shared): State<Shared>,
        Query(query): Query<HashMap<String, String>>,
    ) -> Json<Value> {
        let mut recorded = shared.lock().unwrap();
        recorded.list_queries.push(query);
        Json(Value::Array(recorded.listed_pulls.clone()))
    }

    async fn repo() -> Json<Value> {
        Json(json!({"default_branch": "trunk"}))
    }

    async fn installation(
        State(shared): State<Shared>,
        axum::extract::Path(id): axum::extract::Path<i64>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        shared.lock().unwrap().token_headers.push(headers);
        if id == 404 {
            return (AxumStatus::NOT_FOUND, Json(json!({"message": "Not Found"}))).into_response();
        }
        let kind = shared
            .lock()
            .unwrap()
            .account_kind
            .clone()
            .unwrap_or_else(|| "Organization".to_owned());
        Json(json!({"id": id, "account": {"login": "acme", "type": kind}})).into_response()
    }

    async fn membership(State(shared): State<Shared>) -> impl IntoResponse {
        match shared.lock().unwrap().membership.clone() {
            Some(body) => Json(body).into_response(),
            None => (AxumStatus::NOT_FOUND, Json(json!({"message": "Not Found"}))).into_response(),
        }
    }

    async fn authenticated_user(State(shared): State<Shared>) -> Json<Value> {
        let login = shared
            .lock()
            .unwrap()
            .user_login
            .clone()
            .unwrap_or_default();
        Json(json!({"login": login}))
    }

    async fn git_ref(
        State(shared): State<Shared>,
        axum::extract::Path((_owner, _repo, branch)): axum::extract::Path<(String, String, String)>,
    ) -> impl IntoResponse {
        assert_eq!(branch, "capitainerie/run-1");
        match shared.lock().unwrap().branch_sha.clone() {
            Some(sha) => {
                Json(json!({"ref": format!("refs/heads/{branch}"), "object": {"sha": sha}}))
                    .into_response()
            }
            None => (AxumStatus::NOT_FOUND, Json(json!({"message": "Not Found"}))).into_response(),
        }
    }

    async fn oauth_token(
        State(shared): State<Shared>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        assert_eq!(headers["accept"], "application/json");
        let mut recorded = shared.lock().unwrap();
        recorded.oauth_bodies.push(body);
        Json(
            recorded
                .oauth_reply
                .clone()
                .unwrap_or_else(|| json!({"access_token": "ghu_secret", "token_type": "bearer"})),
        )
    }

    async fn user_installations(
        State(shared): State<Shared>,
        Query(query): Query<HashMap<String, String>>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        let base = {
            let mut recorded = shared.lock().unwrap();
            recorded
                .user_auth
                .push(headers["authorization"].to_str().unwrap().to_owned());
            recorded.base.clone()
        };
        match query.get("page").map(String::as_str) {
            None => (
                [(
                    "link",
                    format!("<{base}/user/installations?per_page=100&page=2>; rel=\"next\""),
                )],
                Json(json!({"total_count": 3, "installations": [{"id": 1}, {"id": 2}]})),
            )
                .into_response(),
            _ => Json(json!({"total_count": 3, "installations": [{"id": 3}]})).into_response(),
        }
    }

    async fn serve(shared: Shared) -> GithubClient {
        let app = Router::new()
            .route("/login/oauth/access_token", post(oauth_token))
            .route("/user/installations", get(user_installations))
            .route("/app/installations/{id}", get(installation))
            .route("/user/memberships/orgs/{org}", get(membership))
            .route("/user", get(authenticated_user))
            .route(
                "/repos/{owner}/{repo}/git/ref/heads/{*branch}",
                get(git_ref),
            )
            .route("/app/installations/{id}/access_tokens", post(issue_token))
            .route("/installation/repositories", get(repositories))
            .route("/repos/{owner}/{repo}", get(repo))
            .route("/repos/{owner}/{repo}/pulls", post(pulls).get(list_pulls))
            .with_state(Arc::clone(&shared));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        shared.lock().unwrap().base = base.clone();
        tokio::spawn(async move { axum::serve(listener, app).await });
        let mut github = GithubApp::new(1, SecretToken::new(PRIVATE));
        github.api_base.clone_from(&base);
        github.oauth_base = base;
        github.client_id = Some("Iv1.abc".into());
        github.client_secret = Some(SecretToken::new("oauth-secret"));
        GithubClient::new(github).unwrap()
    }

    fn repo_ref(value: &str) -> RepositoryRef {
        value.parse().unwrap()
    }

    #[test]
    fn next_link_is_parsed() {
        let header = "<https://x/a?page=1>; rel=\"prev\", <https://x/a?page=3>; rel=\"next\", <https://x/a?page=9>; rel=\"last\"";
        assert_eq!(next_link(header), Some("https://x/a?page=3"));
        assert_eq!(next_link("<https://x/a?page=9>; rel=\"last\""), None);
        assert_eq!(next_link(""), None);
    }

    #[test]
    fn debug_hides_secrets() {
        let token = InstallationToken {
            token: SecretToken::new("ghs_leak"),
            expires_at: Utc::now(),
        };
        assert!(!format!("{token:?}").contains("ghs_leak"));
        let app = GithubApp::new(1, SecretToken::new("pem-leak"));
        assert!(!format!("{app:?}").contains("pem-leak"));
        let mut app = GithubApp::new(1, SecretToken::new("pem"));
        app.client_secret = Some(SecretToken::new("oauth-leak"));
        assert!(!format!("{app:?}").contains("oauth-leak"));
        assert!(!format!("{:?}", UserToken(SecretToken::new("ghu_leak"))).contains("ghu_leak"));
    }

    #[tokio::test]
    async fn the_oauth_code_is_exchanged_for_a_user_token() {
        let shared = Shared::default();
        let client = serve(Arc::clone(&shared)).await;
        let token = client.exchange_user_code("the-code").await.unwrap();
        assert_eq!(token.0.expose(), "ghu_secret");
        assert_eq!(
            shared.lock().unwrap().oauth_bodies[0],
            json!({"client_id": "Iv1.abc", "client_secret": "oauth-secret", "code": "the-code"})
        );
    }

    #[tokio::test]
    async fn a_rejected_oauth_code_is_a_permanent_error() {
        let shared = Shared::default();
        shared.lock().unwrap().oauth_reply = Some(json!({"error": "bad_verification_code"}));
        let client = serve(Arc::clone(&shared)).await;
        let error = client.exchange_user_code("stale").await.unwrap_err();
        assert!(matches!(error, GithubError::Status { status: 400, .. }));
        assert!(matches!(
            CoreError::from(error),
            CoreError::Invalid(message) if message.contains("bad_verification_code")
        ));
    }

    #[tokio::test]
    async fn exchanging_without_oauth_settings_fails() {
        let client = GithubClient::new(GithubApp::new(1, SecretToken::new(PRIVATE))).unwrap();
        assert!(!client.oauth_configured());
        assert!(matches!(
            client.exchange_user_code("c").await,
            Err(GithubError::OAuthNotConfigured)
        ));
    }

    #[tokio::test]
    async fn user_installations_are_searched_across_pages_with_the_user_token() {
        let shared = Shared::default();
        let client = serve(Arc::clone(&shared)).await;
        let token = client.exchange_user_code("c").await.unwrap();
        assert!(
            client
                .user_can_access_installation(&token, 3)
                .await
                .unwrap()
        );
        assert!(
            client
                .user_can_access_installation(&token, 1)
                .await
                .unwrap()
        );
        assert!(
            !client
                .user_can_access_installation(&token, 9)
                .await
                .unwrap()
        );
        let recorded = shared.lock().unwrap();
        assert!(
            recorded
                .user_auth
                .iter()
                .all(|auth| auth == "Bearer ghu_secret")
        );
    }

    #[test]
    fn errors_map_to_core_errors() {
        let status = |status| {
            CoreError::from(GithubError::Status {
                status,
                message: "m".into(),
            })
        };
        for transient in [429, 500, 502] {
            assert!(matches!(status(transient), CoreError::Internal(_)));
        }
        for permanent in [401, 403, 404, 422] {
            assert!(matches!(status(permanent), CoreError::Invalid(_)));
        }
        assert!(matches!(
            CoreError::from(GithubError::Transport("down".into())),
            CoreError::Internal(_)
        ));
    }

    #[tokio::test]
    async fn token_request_is_scoped_and_cached() {
        let shared = Shared::default();
        let client = serve(Arc::clone(&shared)).await;
        let api = repo_ref("acme/api");

        let first = client.installation_token(5, &api).await.unwrap();
        let second = client.installation_token(5, &api).await.unwrap();
        assert_eq!(first.token.expose(), "ghs_secret1");
        assert_eq!(second.token.expose(), "ghs_secret1");
        client
            .installation_token(5, &repo_ref("acme/web"))
            .await
            .unwrap();

        let recorded = shared.lock().unwrap();
        assert_eq!(recorded.token_bodies.len(), 2);
        assert_eq!(
            recorded.token_bodies[0],
            json!({
                "repositories": ["api"],
                "permissions": {"contents": "write", "pull_requests": "write"},
            })
        );
        let headers = &recorded.token_headers[0];
        assert!(
            headers["authorization"]
                .to_str()
                .unwrap()
                .starts_with("Bearer ey")
        );
        assert_eq!(headers["accept"], "application/vnd.github+json");
        assert_eq!(headers["x-github-api-version"], "2022-11-28");
        assert!(headers.contains_key("user-agent"));
    }

    #[tokio::test]
    async fn token_close_to_expiry_is_renewed() {
        let shared = Shared::default();
        shared.lock().unwrap().expires_at = Some((Utc::now() + Duration::minutes(4)).to_rfc3339());
        let client = serve(Arc::clone(&shared)).await;
        let api = repo_ref("acme/api");
        client.installation_token(5, &api).await.unwrap();
        client.installation_token(5, &api).await.unwrap();
        assert_eq!(shared.lock().unwrap().token_bodies.len(), 2);
    }

    #[tokio::test]
    async fn repositories_follow_pagination_with_an_unscoped_token() {
        let shared = Shared::default();
        let client = serve(Arc::clone(&shared)).await;
        let repositories = client.repositories(5).await.unwrap();
        let names: Vec<_> = repositories
            .iter()
            .map(|r| format!("{}/{}", r.owner(), r.name()))
            .collect();
        assert_eq!(names, ["acme/api", "acme/web", "acme/docs"]);
        let recorded = shared.lock().unwrap();
        assert!(recorded.token_bodies[0].get("repositories").is_none());
    }

    #[tokio::test]
    async fn installation_account_is_read_with_the_app_jwt() {
        let shared = Shared::default();
        let client = serve(Arc::clone(&shared)).await;
        let account = client.installation_account(5).await.unwrap();
        assert_eq!(account.login, "acme");
        assert_eq!(account.kind, AccountKind::Organization);
        {
            let recorded = shared.lock().unwrap();
            assert!(
                recorded.token_headers[0]["authorization"]
                    .to_str()
                    .unwrap()
                    .starts_with("Bearer ey")
            );
        }
        let error = client.installation_account(404).await.unwrap_err();
        assert!(matches!(error, GithubError::Status { status: 404, .. }));
    }

    async fn owns(shared: &Shared, kind: &str) -> bool {
        shared.lock().unwrap().account_kind = Some(kind.to_owned());
        let client = serve(Arc::clone(shared)).await;
        let token = client.exchange_user_code("c").await.unwrap();
        let account = client.installation_account(5).await.unwrap();
        client
            .user_owns_installation(&token, &account)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn only_an_active_org_admin_owns_an_organization_installation() {
        let admin = Shared::default();
        admin.lock().unwrap().membership = Some(json!({"role": "admin", "state": "active"}));
        assert!(owns(&admin, "Organization").await);

        let member = Shared::default();
        member.lock().unwrap().membership = Some(json!({"role": "member", "state": "active"}));
        assert!(!owns(&member, "Organization").await);

        let pending = Shared::default();
        pending.lock().unwrap().membership = Some(json!({"role": "admin", "state": "pending"}));
        assert!(!owns(&pending, "Organization").await);

        let outsider = Shared::default();
        assert!(!owns(&outsider, "Organization").await);
    }

    #[tokio::test]
    async fn a_user_installation_is_owned_by_that_user_only() {
        let same = Shared::default();
        same.lock().unwrap().user_login = Some("ACME".into());
        assert!(owns(&same, "User").await);

        let other = Shared::default();
        other.lock().unwrap().user_login = Some("mallory".into());
        assert!(!owns(&other, "User").await);
    }

    #[tokio::test]
    async fn any_other_account_kind_is_not_owned() {
        let shared = Shared::default();
        shared.lock().unwrap().user_login = Some("acme".into());
        assert!(!owns(&shared, "Bot").await);
    }

    #[tokio::test]
    async fn the_head_of_a_branch_is_read_and_a_missing_branch_is_none() {
        let shared = Shared::default();
        shared.lock().unwrap().branch_sha = Some("abc123".into());
        let client = serve(Arc::clone(&shared)).await;
        let api = repo_ref("acme/api");

        assert_eq!(
            client
                .branch_head(5, &api, "capitainerie/run-1")
                .await
                .unwrap(),
            Some("abc123".to_owned())
        );
        shared.lock().unwrap().branch_sha = None;
        assert_eq!(
            client
                .branch_head(5, &api, "capitainerie/run-1")
                .await
                .unwrap(),
            None
        );
    }

    #[test]
    fn a_link_must_share_scheme_host_and_port_with_the_api() {
        let base = "https://api.github.com";
        assert!(same_origin(
            "https://api.github.com/user/installations?page=2",
            base
        ));
        assert!(same_origin("https://API.github.com:443/x", base));
        assert!(!same_origin("https://api.github.com.evil.example/x", base));
        assert!(!same_origin("https://api.github.com@evil.example/x", base));
        assert!(!same_origin("http://api.github.com/x", base));
        assert!(!same_origin("https://api.github.com:8443/x", base));
        assert!(!same_origin("not a url", base));
        assert!(same_origin(
            "http://127.0.0.1:4000/a",
            "http://127.0.0.1:4000"
        ));
    }

    #[tokio::test]
    async fn default_branch_is_read() {
        let client = serve(Shared::default()).await;
        assert_eq!(
            client
                .default_branch(5, &repo_ref("acme/api"))
                .await
                .unwrap(),
            "trunk"
        );
    }

    fn draft() -> DraftPullRequest {
        DraftPullRequest {
            repository: repo_ref("acme/api"),
            base: "main".into(),
            head: "capitainerie/run-1".into(),
            title: "Fix it".into(),
            body: "details".into(),
        }
    }

    #[tokio::test]
    async fn draft_pull_request_is_created() {
        let shared = Shared::default();
        let client = serve(Arc::clone(&shared)).await;
        let url = client.open_draft_pr(5, draft()).await.unwrap();
        assert_eq!(
            url,
            PullRequestUrl("https://github.com/acme/api/pull/9".into())
        );
        assert_eq!(
            shared.lock().unwrap().pull_bodies[0],
            json!({
                "title": "Fix it",
                "head": "capitainerie/run-1",
                "base": "main",
                "body": "details",
                "draft": true,
            })
        );
    }

    #[tokio::test]
    async fn an_existing_pull_request_for_the_branch_is_reused() {
        let shared = Shared::default();
        {
            let mut recorded = shared.lock().unwrap();
            recorded.pull_exists = true;
            recorded.listed_pulls = vec![json!({"html_url": "https://github.com/acme/api/pull/4"})];
        }
        let client = serve(Arc::clone(&shared)).await;

        let url = client.open_draft_pr(5, draft()).await.unwrap();

        assert_eq!(
            url,
            PullRequestUrl("https://github.com/acme/api/pull/4".into())
        );
        let recorded = shared.lock().unwrap();
        assert_eq!(recorded.list_queries.len(), 1);
        assert_eq!(recorded.list_queries[0]["head"], "acme:capitainerie/run-1");
        assert_eq!(recorded.list_queries[0]["state"], "open");
    }

    #[tokio::test]
    async fn a_duplicate_error_without_an_open_pull_request_stays_an_error() {
        let shared = Shared::default();
        shared.lock().unwrap().pull_exists = true;
        let client = serve(Arc::clone(&shared)).await;

        let error = client.open_draft_pr(5, draft()).await.unwrap_err();

        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn github_failures_are_mapped_without_leaking_tokens() {
        let shared = Shared::default();
        let client = serve(Arc::clone(&shared)).await;
        for (status, transient) in [
            (422, false),
            (404, false),
            (403, false),
            (503, true),
            (429, true),
        ] {
            shared.lock().unwrap().fail_pulls_with = Some(status);
            let error = client.open_draft_pr(5, draft()).await.unwrap_err();
            let text = error.to_string();
            assert!(!text.contains("ghs_secret"));
            if transient {
                assert!(matches!(error, CoreError::Internal(_)), "{status}");
            } else {
                assert!(text.contains("Validation Failed"));
                assert!(matches!(error, CoreError::Invalid(_)), "{status}");
            }
        }
    }

    #[tokio::test]
    async fn unreachable_host_is_transient() {
        let mut app = GithubApp::new(1, SecretToken::new(PRIVATE));
        app.api_base = "http://127.0.0.1:1".into();
        let client = GithubClient::new(app).unwrap();
        let error = client.repositories(5).await.unwrap_err();
        assert!(matches!(error, CoreError::Internal(_)));
    }
}
