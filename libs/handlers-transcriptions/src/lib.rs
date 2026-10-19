pub mod create_transcription;
pub mod state;

use axum::{Router, extract::DefaultBodyLimit};
use axum_extra::routing::RouterExt;
use capitainerie_core::{
    CapitainerieUseCase,
    domain::transcription::{MAX_AUDIO_BYTES, ports::TranscriberFactory},
};

use crate::state::TranscriptionsState;

pub const TAG: &str = "transcriptions";

pub const MULTIPART_OVERHEAD: usize = 64 * 1024;

pub fn router<S, F>(usecase: CapitainerieUseCase, factory: F) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    F: TranscriberFactory + 'static,
{
    Router::<TranscriptionsState<F>>::new()
        .typed_post(create_transcription::handler::<F>)
        .layer(DefaultBodyLimit::max(MAX_AUDIO_BYTES + MULTIPART_OVERHEAD))
        .with_state(TranscriptionsState::new(usecase, factory))
}
