pub mod me;

use axum::{Router, routing::get};
use capitainerie_handlers::AppState;

pub const TAG: &str = "identity";

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/me", get(me::handler))
}
