pub mod decide_approval;
pub mod get_approval;
pub mod list_approvals;
pub mod response;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_handlers::AppState;

pub const TAG: &str = "approvals";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(list_approvals::handler)
        .typed_get(get_approval::handler)
        .typed_post(decide_approval::handler)
}
