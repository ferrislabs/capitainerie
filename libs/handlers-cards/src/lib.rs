pub mod auto_run;
pub mod comment_card;
pub mod create_card;
pub mod execute_card;
pub mod get_card;
pub mod intervene_card;
pub mod list_cards;
pub mod move_card;
pub mod patch_card;
pub mod response;
pub mod run_agent;
pub mod worksites;

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
        .typed_post(create_card::handler)
        .typed_get(get_card::handler)
        .typed_patch(patch_card::handler)
        .typed_post(move_card::handler)
        .typed_post(comment_card::handler)
        .typed_post(execute_card::handler)
        .typed_post(intervene_card::handler)
        .typed_post(run_agent::handler)
        .typed_get(worksites::list_handler)
        .typed_post(worksites::launch_handler)
        .layer(Extension(CodeHostHandle(github)))
}
