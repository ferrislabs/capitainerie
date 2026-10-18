use std::{collections::HashMap, fmt, time::Duration};

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        execution::{
            DraftPullRequest, GithubAppCredentials, PullRequestUrl, RepositoryRef, ports::CodeHost,
        },
    },
};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use tokio::{sync::Mutex, time::Instant};

use crate::{
    GithubApp, GithubBases, GithubClient, GithubError,
    client::{AGENT, API_VERSION},
};

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const APP_MISSING: &str = "application GitHub absente";

#[derive(Deserialize)]
struct ConversionBody {
    id: i64,
    slug: String,
    client_id: String,
    client_secret: String,
    pem: String,
    webhook_secret: Option<String>,
    html_url: String,
    owner: ConversionOwner,
}

#[derive(Deserialize)]
struct ConversionOwner {
    login: String,
    #[serde(rename = "type")]
    kind: String,
}

pub struct ManifestConversion {
    pub app_id: i64,
    pub slug: String,
    pub client_id: String,
    pub client_secret: SecretToken,
    pub private_key_pem: SecretToken,
    pub webhook_secret: SecretToken,
    pub html_url: String,
    pub owner_login: String,
    pub owner_type: String,
}

impl fmt::Debug for ManifestConversion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManifestConversion")
            .field("app_id", &self.app_id)
            .field("slug", &self.slug)
            .field("client_id", &self.client_id)
            .field("html_url", &self.html_url)
            .field("owner_login", &self.owner_login)
            .field("owner_type", &self.owner_type)
            .finish_non_exhaustive()
    }
}

impl ManifestConversion {
    pub fn into_credentials(
        self,
        organization_id: OrganizationId,
        created_by: String,
    ) -> GithubAppCredentials {
        GithubAppCredentials {
            organization_id,
            app_id: self.app_id,
            slug: self.slug,
            client_id: self.client_id,
            client_secret: self.client_secret,
            private_key_pem: self.private_key_pem,
            webhook_secret: self.webhook_secret,
            html_url: self.html_url,
            owner_login: self.owner_login,
            owner_type: self.owner_type,
            created_by,
            created_at: chrono::Utc::now(),
        }
    }
}

struct Cached {
    app_id: i64,
    client: GithubClient,
    checked_at: Instant,
}

pub struct GithubApps {
    usecase: CapitainerieUseCase,
    bases: GithubBases,
    http: reqwest::Client,
    ttl: Duration,
    cache: Mutex<HashMap<OrganizationId, Cached>>,
}

impl GithubApps {
    pub fn new(usecase: CapitainerieUseCase, bases: GithubBases) -> Result<Self, GithubError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(GithubError::transport)?;
        Ok(Self {
            usecase,
            bases,
            http,
            ttl: CACHE_TTL,
            cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn bases(&self) -> &GithubBases {
        &self.bases
    }

    pub async fn client(
        &self,
        organization_id: OrganizationId,
    ) -> Result<Option<GithubClient>, CoreError> {
        if let Some(cached) = self.cache.lock().await.get(&organization_id)
            && cached.checked_at.elapsed() < self.ttl
        {
            return Ok(Some(cached.client.clone()));
        }
        let Some(credentials) = self.usecase.github_app_for(organization_id).await? else {
            self.cache.lock().await.remove(&organization_id);
            return Ok(None);
        };
        let mut cache = self.cache.lock().await;
        if let Some(cached) = cache.get_mut(&organization_id)
            && cached.app_id == credentials.app_id
        {
            cached.checked_at = Instant::now();
            return Ok(Some(cached.client.clone()));
        }
        let app = GithubApp::new(&credentials, &self.bases)?;
        let client = GithubClient::with_http(app, self.http.clone());
        cache.insert(
            organization_id,
            Cached {
                app_id: credentials.app_id,
                client: client.clone(),
                checked_at: Instant::now(),
            },
        );
        Ok(Some(client))
    }

    async fn required(&self, organization_id: OrganizationId) -> Result<GithubClient, CoreError> {
        self.client(organization_id)
            .await?
            .ok_or_else(|| CoreError::Invalid(APP_MISSING.into()))
    }

    pub async fn default_branch(
        &self,
        organization_id: OrganizationId,
        installation_id: i64,
        repository: &RepositoryRef,
    ) -> Result<String, CoreError> {
        Ok(self
            .required(organization_id)
            .await?
            .default_branch(installation_id, repository)
            .await?)
    }

    pub async fn convert_manifest(&self, code: &str) -> Result<ManifestConversion, GithubError> {
        if code.is_empty()
            || code.len() > 128
            || !code
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err(GithubError::Status {
                status: 400,
                message: "the manifest code is malformed".into(),
            });
        }
        let url = format!(
            "{}/app-manifests/{code}/conversions",
            self.bases.api_base.trim_end_matches('/')
        );
        let response = GithubClient::send(
            self.http
                .post(url)
                .header(ACCEPT, "application/vnd.github+json")
                .header("X-GitHub-Api-Version", API_VERSION)
                .header(USER_AGENT, AGENT),
        )
        .await?;
        let body: ConversionBody = GithubClient::decode(response).await?;
        Ok(ManifestConversion {
            app_id: body.id,
            slug: body.slug,
            client_id: body.client_id,
            client_secret: SecretToken::new(body.client_secret),
            private_key_pem: SecretToken::new(body.pem),
            webhook_secret: SecretToken::new(body.webhook_secret.unwrap_or_default()),
            html_url: body.html_url,
            owner_login: body.owner.login,
            owner_type: body.owner.kind,
        })
    }
}

impl CodeHost for GithubApps {
    async fn repositories(
        &self,
        organization_id: OrganizationId,
        installation_id: i64,
    ) -> Result<Vec<RepositoryRef>, CoreError> {
        Ok(self
            .required(organization_id)
            .await?
            .list_repositories(installation_id)
            .await?)
    }

    async fn branch_head(
        &self,
        organization_id: OrganizationId,
        installation_id: i64,
        repository: &RepositoryRef,
        branch: &str,
    ) -> Result<Option<String>, CoreError> {
        Ok(self
            .required(organization_id)
            .await?
            .branch_sha(installation_id, repository, branch)
            .await?)
    }

    async fn open_draft_pr(
        &self,
        organization_id: OrganizationId,
        installation_id: i64,
        pull_request: DraftPullRequest,
    ) -> Result<PullRequestUrl, CoreError> {
        Ok(self
            .required(organization_id)
            .await?
            .create_draft_pr(installation_id, &pull_request)
            .await?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use axum::{Json, Router, extract::Path, http::StatusCode, routing::post};
    use serde_json::{Value, json};

    use super::*;

    async fn conversions(Path(code): Path<String>) -> (StatusCode, Json<Value>) {
        if code == "expired" {
            return (StatusCode::NOT_FOUND, Json(json!({"message": "Not Found"})));
        }
        (
            StatusCode::CREATED,
            Json(json!({
                "id": 4242,
                "slug": "capitainerie-acme-1a2b3c",
                "client_id": "Iv1.client",
                "client_secret": "oauth-secret-leak",
                "pem": "-----BEGIN RSA PRIVATE KEY-----\nleak\n-----END RSA PRIVATE KEY-----\n",
                "webhook_secret": null,
                "html_url": "https://github.com/apps/capitainerie-acme-1a2b3c",
                "owner": {"login": "acme", "type": "Organization"},
            })),
        )
    }

    async fn apps_against_fake() -> GithubApps {
        let app = Router::new().route("/app-manifests/{code}/conversions", post(conversions));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        let pool = sqlx::PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/none").unwrap();
        GithubApps::new(
            CapitainerieUseCase::with_local_policies(pool),
            GithubBases {
                api_base: base.clone(),
                web_base: base,
            },
        )
        .unwrap()
    }

    #[tokio::test]
    async fn a_manifest_code_is_converted_into_the_app_credentials() {
        let apps = apps_against_fake().await;
        let conversion = apps.convert_manifest("abc-123_x").await.unwrap();
        assert_eq!(conversion.app_id, 4242);
        assert_eq!(conversion.slug, "capitainerie-acme-1a2b3c");
        assert_eq!(conversion.client_id, "Iv1.client");
        assert_eq!(conversion.client_secret.expose(), "oauth-secret-leak");
        assert!(conversion.private_key_pem.expose().contains("PRIVATE KEY"));
        assert_eq!(conversion.webhook_secret.expose(), "");
        assert_eq!(conversion.owner_login, "acme");
        assert_eq!(conversion.owner_type, "Organization");
        let shown = format!("{conversion:?}");
        assert!(!shown.contains("leak"), "{shown}");
    }

    #[tokio::test]
    async fn an_expired_code_is_a_permanent_error() {
        let apps = apps_against_fake().await;
        let error = apps.convert_manifest("expired").await.unwrap_err();
        assert!(matches!(error, GithubError::Status { status: 404, .. }));
        assert!(matches!(CoreError::from(error), CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn a_code_that_could_alter_the_url_is_refused_before_any_call() {
        let apps = apps_against_fake().await;
        for code in ["", "a/b", "a?b", "..", "a b", "é"] {
            assert!(
                matches!(
                    apps.convert_manifest(code).await,
                    Err(GithubError::Status { status: 400, .. })
                ),
                "{code:?}"
            );
        }
    }
}
