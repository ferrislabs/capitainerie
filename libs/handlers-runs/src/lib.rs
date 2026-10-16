pub mod get_run;
pub mod list_runs;
pub mod response;
pub mod submit_transcript;
pub mod timeline;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "runs";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_post(submit_transcript::handler)
        .typed_get(list_runs::handler)
        .typed_get(get_run::handler)
        .typed_get(timeline::handler)
}
