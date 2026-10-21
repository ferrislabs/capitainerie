pub mod actions;
pub mod callback;
pub mod create_app;
pub mod create_manifest;
pub mod delete_app;
pub mod disconnect;
pub mod dto;
pub mod error;
pub mod get_integration;
pub mod install_url;
pub mod instances;
pub mod list_repositories;
mod manifest;
pub mod signer;
pub mod state;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_core::CapitainerieUseCase;
use capitainerie_integrations::HttpConnector;

pub use actions::dto::{
    ActionAccessDto, ActionOutcomeDto, IntegrationActionListResponse, IntegrationActionResponse,
};
pub use dto::{
    ConnectGithubRequest, CreateGithubAppRequest, CreateManifestRequest, GithubAppResponse,
    GithubInstallationResponse, GithubIntegrationResponse, InstallUrlResponse, ManifestOwner,
    ManifestResponse, RepositoryListResponse, RepositoryResponse,
};
pub use error::IntegrationError;
pub use instances::dto::{
    CheckIntegrationInstanceResponse, CreateIntegrationInstanceRequest, InstanceFailureResponse,
    IntegrationEnvironmentDto, IntegrationInstanceListResponse, IntegrationInstanceResponse,
    IntegrationKindDto, UpdateIntegrationInstanceRequest,
};
pub use state::{GithubIntegration, IntegrationsState};

pub const TAG: &str = "integrations";

pub fn router<S>(
    usecase: CapitainerieUseCase,
    github: GithubIntegration,
    connector: HttpConnector,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<IntegrationsState>::new()
        .typed_get(get_integration::handler)
        .typed_delete(disconnect::handler)
        .typed_post(create_manifest::handler)
        .typed_post(create_app::handler)
        .typed_delete(delete_app::handler)
        .typed_get(install_url::handler)
        .typed_post(callback::handler)
        .typed_get(list_repositories::handler)
        .typed_get(instances::list::handler)
        .typed_post(instances::create::handler)
        .typed_patch(instances::update::handler)
        .typed_delete(instances::archive::handler)
        .typed_post(instances::check::handler)
        .typed_get(actions::list_for_card::handler)
        .typed_get(actions::list_for_run::handler)
        .with_state(IntegrationsState {
            usecase,
            github,
            connector,
        })
}
