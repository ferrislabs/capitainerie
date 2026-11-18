pub mod archive_skill;
pub mod create_skill;
pub mod get_skill;
pub mod list_skills;
pub mod response;
pub mod update_skill;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "skills";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(list_skills::handler)
        .typed_post(create_skill::handler)
        .typed_get(get_skill::handler)
        .typed_patch(update_skill::handler)
        .typed_post(archive_skill::handler)
}
