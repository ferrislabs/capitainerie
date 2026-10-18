use capitainerie_core::domain::execution::{GithubAppSummary, GithubInstallation, RepositoryRef};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct GithubAppResponse {
    pub slug: String,
    pub owner_login: String,
    /// `User` or `Organization`, as GitHub names the account the App belongs to.
    pub owner_type: String,
    /// The App's page on GitHub: where to delete it for good.
    pub html_url: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl From<GithubAppSummary> for GithubAppResponse {
    fn from(app: GithubAppSummary) -> Self {
        Self {
            slug: app.slug,
            owner_login: app.owner_login,
            owner_type: app.owner_type,
            html_url: app.html_url,
            created_by: app.created_by,
            created_at: app.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GithubInstallationResponse {
    pub account_login: String,
    pub installation_id: i64,
    pub connected_by: String,
    pub connected_at: DateTime<Utc>,
}

impl From<GithubInstallation> for GithubInstallationResponse {
    fn from(installation: GithubInstallation) -> Self {
        Self {
            account_login: installation.account_login,
            installation_id: installation.installation_id,
            connected_by: installation.created_by,
            connected_at: installation.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GithubIntegrationResponse {
    /// The GitHub App the organization created; `null` until it does.
    pub app: Option<GithubAppResponse>,
    /// Where that App is installed; `null` until an admin installs it.
    pub installation: Option<GithubInstallationResponse>,
}

impl GithubIntegrationResponse {
    pub fn new(app: Option<GithubAppSummary>, installation: Option<GithubInstallation>) -> Self {
        Self {
            app: app.map(Into::into),
            installation: installation.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstallUrlResponse {
    pub url: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectGithubRequest {
    pub installation_id: i64,
    /// The OAuth `code` GitHub returns with the redirect; proves the admin can access the installation.
    pub code: String,
    /// The `state` of the install URL, returned untouched by GitHub.
    pub state: String,
}

/// Who will own the App on GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ManifestOwner {
    User,
    Organization { login: String },
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateManifestRequest {
    pub owner: ManifestOwner,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ManifestResponse {
    /// Where the browser POSTs the `manifest` form field.
    pub action_url: String,
    /// The App manifest, a JSON document sent as the `manifest` form field.
    pub manifest: String,
    /// Signed, bound to the caller and the organization, valid 10 minutes.
    pub state: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGithubAppRequest {
    /// The temporary `code` GitHub appends to the redirect URL.
    pub code: String,
    /// The `state` of the manifest, returned untouched by GitHub.
    pub state: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RepositoryResponse {
    pub owner: String,
    pub name: String,
    pub full_name: String,
}

impl From<&RepositoryRef> for RepositoryResponse {
    fn from(repository: &RepositoryRef) -> Self {
        Self {
            owner: repository.owner().to_owned(),
            name: repository.name().to_owned(),
            full_name: repository.to_string(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RepositoryListResponse {
    pub repositories: Vec<RepositoryResponse>,
}
