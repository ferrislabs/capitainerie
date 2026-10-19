pub mod create_client;
pub mod create_project;
pub mod get_client;
pub mod get_project;
pub mod list_clients;
pub mod list_projects;
pub mod response;
pub mod update_client;
pub mod update_project;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "clients";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(list_clients::handler)
        .typed_post(create_client::handler)
        .typed_get(get_client::handler)
        .typed_patch(update_client::handler)
        .typed_get(list_projects::handler)
        .typed_post(create_project::handler)
        .typed_get(get_project::handler)
        .typed_patch(update_project::handler)
}
