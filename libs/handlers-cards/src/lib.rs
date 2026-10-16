pub mod list_cards;
pub mod move_card;
pub mod response;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "cards";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(list_cards::handler)
        .typed_post(move_card::handler)
}
