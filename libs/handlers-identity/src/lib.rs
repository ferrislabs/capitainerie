pub mod list_members;
pub mod me;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "identity";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(me::handler)
        .typed_get(list_members::handler)
}
