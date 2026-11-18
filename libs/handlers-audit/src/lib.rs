pub mod budgets;
pub mod events;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "audit";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(events::handler)
        .typed_get(budgets::get_handler)
        .typed_put(budgets::put_handler)
}
