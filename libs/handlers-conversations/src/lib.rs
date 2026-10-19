pub mod create_conversation;
pub mod dto;
pub mod get_conversation;
pub mod list_conversations;
pub mod send_message;
pub mod state;
pub mod tools;
pub mod update_conversation;

use axum::Router;
use axum_extra::routing::RouterExt;
use capitainerie_core::domain::{
    conversation::turn::CaptainRuntime, delegation::ports::TokenExchanger,
    execution::ports::CodeHost,
};

pub use state::ConversationsState;

pub const TAG: &str = "conversations";

pub fn router<S, R, H, X>(state: ConversationsState<R, H, X>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    R: CaptainRuntime + 'static,
    H: CodeHost + 'static,
    X: TokenExchanger + 'static,
{
    Router::<ConversationsState<R, H, X>>::new()
        .typed_post(create_conversation::handler::<R, H, X>)
        .typed_get(list_conversations::handler::<R, H, X>)
        .typed_get(get_conversation::handler::<R, H, X>)
        .typed_patch(update_conversation::handler::<R, H, X>)
        .typed_post(send_message::handler::<R, H, X>)
        .with_state(state)
}
