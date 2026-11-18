pub mod archive_source;
pub mod create_source;
pub mod get_source;
pub mod hook;
pub mod list_events;
pub mod list_sources;
pub mod response;
pub mod rotate_token;
pub mod test_source;
pub mod update_source;

use axum::{Extension, Router, extract::DefaultBodyLimit};
use axum_extra::routing::RouterExt;
use capitainerie_core::domain::source::PAYLOAD_MAX_BYTES;
use capitainerie_handlers::AppState;

pub const TAG: &str = "sources";

pub use capitainerie_handlers::PublicBaseUrl;

/// The authenticated management routes.
pub fn router(public_base_url: Option<String>) -> Router<AppState> {
    Router::new()
        .typed_get(list_sources::handler)
        .typed_post(create_source::handler)
        .typed_get(get_source::handler)
        .typed_patch(update_source::handler)
        .typed_post(archive_source::handler)
        .typed_post(rotate_token::handler)
        .typed_get(list_events::handler)
        .typed_post(test_source::handler)
        .layer(Extension(PublicBaseUrl(public_base_url)))
}

/// The public hook: no bearer token, the path's token designates the source.
pub fn hooks_router() -> Router<AppState> {
    Router::new()
        .typed_post(hook::handler)
        .layer(DefaultBodyLimit::max(PAYLOAD_MAX_BYTES))
}
