pub mod authorize;
pub mod callback;
pub mod create_connector;
pub mod disconnect;
pub mod get_connector;
pub mod list_connectors;
pub mod response;
pub mod state;
pub mod test_connector;
pub mod update_connector;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_core::{
    CapitainerieUseCase,
    domain::mcp::ports::{McpGateway, McpOAuth},
};

pub use state::McpState;

pub const TAG: &str = "mcp-connectors";

pub fn router<S, A>(
    usecase: CapitainerieUseCase,
    adapter: A,
    console_url: Option<String>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    A: McpOAuth + McpGateway + 'static,
{
    Router::<McpState<A>>::new()
        .typed_get(list_connectors::handler::<A>)
        .typed_post(create_connector::handler::<A>)
        .typed_get(get_connector::handler::<A>)
        .typed_patch(update_connector::handler::<A>)
        .typed_post(authorize::handler::<A>)
        .typed_post(callback::handler::<A>)
        .typed_delete(disconnect::handler::<A>)
        .typed_post(test_connector::handler::<A>)
        .with_state(McpState::new(usecase, adapter, console_url))
}
