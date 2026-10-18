use capitainerie_core::domain::execution::{GithubInstallation, RepositoryRef};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct GithubIntegrationResponse {
    pub connected: bool,
    /// False when the server has no GitHub App: nothing can then be connected.
    pub app_configured: bool,
    pub account_login: Option<String>,
    pub installation_id: Option<i64>,
    pub connected_by: Option<String>,
    pub connected_at: Option<DateTime<Utc>>,
}

impl GithubIntegrationResponse {
    pub fn new(installation: Option<GithubInstallation>, app_configured: bool) -> Self {
        match installation {
            Some(installation) => Self {
                connected: true,
                app_configured,
                account_login: Some(installation.account_login),
                installation_id: Some(installation.installation_id),
                connected_by: Some(installation.created_by),
                connected_at: Some(installation.created_at),
            },
            None => Self {
                connected: false,
                app_configured,
                account_login: None,
                installation_id: None,
                connected_by: None,
                connected_at: None,
            },
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
