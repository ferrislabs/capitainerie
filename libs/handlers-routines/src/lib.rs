pub mod hook;
pub mod response;
pub mod routines;
pub mod runs;
pub mod triggers;

use axum::{Extension, Router, extract::DefaultBodyLimit};
use axum_extra::routing::RouterExt;
use capitainerie_handlers::{AppState, PublicBaseUrl};

pub const TAG: &str = "routines";
/// Where a webhook trigger's URL starts, before its token.
pub const HOOK_PREFIX: &str = "/hooks/routines/";
const HOOK_BODY_MAX_BYTES: usize = 64 * 1024;

/// The authenticated management routes.
pub fn router(public_base_url: Option<String>) -> Router<AppState> {
    Router::new()
        .typed_get(routines::list_routines)
        .typed_post(routines::create_routine)
        .typed_get(routines::get_routine)
        .typed_patch(routines::update_routine)
        .typed_post(routines::archive_routine)
        .typed_post(runs::launch_routine)
        .typed_get(runs::list_routine_runs)
        .typed_post(triggers::add_trigger)
        .typed_patch(triggers::update_trigger)
        .typed_delete(triggers::remove_trigger)
        .typed_post(triggers::rotate_trigger_token)
        .layer(Extension(PublicBaseUrl(public_base_url)))
}

/// The public hook: no bearer token, the path's token designates the trigger.
pub fn hooks_router() -> Router<AppState> {
    Router::new()
        .typed_post(hook::trigger_routine)
        .layer(DefaultBodyLimit::max(HOOK_BODY_MAX_BYTES))
}
