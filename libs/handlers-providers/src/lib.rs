pub mod delete_provider;
pub mod dto;
pub mod get_assignments;
pub mod list_models;
pub mod list_providers;
pub mod put_assignments;
pub mod put_provider;
pub mod state;
pub mod test_provider;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_core::{CapitainerieUseCase, domain::provider::ports::ModelCatalog};

use crate::state::ProvidersState;

pub const TAG: &str = "providers";

pub fn router<S, C>(usecase: CapitainerieUseCase, catalog: C) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    C: ModelCatalog + 'static,
{
    Router::<ProvidersState<C>>::new()
        .typed_get(list_providers::handler::<C>)
        .typed_put(put_provider::handler::<C>)
        .typed_delete(delete_provider::handler::<C>)
        .typed_post(test_provider::handler::<C>)
        .typed_get(list_models::handler::<C>)
        .typed_get(get_assignments::handler::<C>)
        .typed_put(put_assignments::handler::<C>)
        .with_state(ProvidersState::new(usecase, catalog))
}
