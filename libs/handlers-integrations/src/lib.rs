pub mod callback;
pub mod disconnect;
pub mod dto;
pub mod error;
pub mod get_integration;
pub mod install_url;
pub mod list_repositories;
pub mod signer;
pub mod state;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_core::CapitainerieUseCase;

pub use dto::{
    ConnectGithubRequest, GithubIntegrationResponse, InstallUrlResponse, RepositoryListResponse,
    RepositoryResponse,
};
pub use error::IntegrationError;
pub use state::{GithubIntegration, IntegrationsState};

pub const TAG: &str = "integrations";

pub fn router<S>(usecase: CapitainerieUseCase, github: GithubIntegration) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<IntegrationsState>::new()
        .typed_get(get_integration::handler)
        .typed_delete(disconnect::handler)
        .typed_get(install_url::handler)
        .typed_post(callback::handler)
        .typed_get(list_repositories::handler)
        .with_state(IntegrationsState { usecase, github })
}
