pub mod create_agent;
pub mod get_agent;
pub mod list_agents;
pub mod response;
pub mod update_agent;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "team";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(list_agents::handler)
        .typed_post(create_agent::handler)
        .typed_get(get_agent::handler)
        .typed_patch(update_agent::handler)
}
