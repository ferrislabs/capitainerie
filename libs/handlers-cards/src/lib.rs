pub mod comment_card;
pub mod execute_card;
pub mod get_card;
pub mod list_cards;
pub mod move_card;
pub mod response;

use std::sync::Arc;

use axum::{Extension, Router};
use axum_extra::routing::RouterExt;
use capitainerie_github::GithubApps;
use capitainerie_handlers::AppState;

use crate::execute_card::CodeHostHandle;

pub const TAG: &str = "cards";

pub fn router(github: Arc<GithubApps>) -> Router<AppState> {
    Router::new()
        .typed_get(list_cards::handler)
        .typed_get(get_card::handler)
        .typed_post(move_card::handler)
        .typed_post(comment_card::handler)
        .typed_post(execute_card::handler)
        .layer(Extension(CodeHostHandle(github)))
}
