pub mod create_transcription;
pub mod documents_state;
pub mod read_document;
pub mod state;

use axum::{Router, extract::DefaultBodyLimit};
use axum_extra::routing::RouterExt;
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        document::{MAX_PDF_BYTES, ports::DocumentReaderFactory},
        transcription::{MAX_AUDIO_BYTES, ports::TranscriberFactory},
    },
};

use crate::{documents_state::DocumentsState, state::TranscriptionsState};

pub const TAG: &str = "transcriptions";

pub const MULTIPART_OVERHEAD: usize = 64 * 1024;

pub fn router<S, F, R>(usecase: CapitainerieUseCase, factory: F, readers: R) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    F: TranscriberFactory + 'static,
    R: DocumentReaderFactory + 'static,
{
    let transcriptions = Router::<TranscriptionsState<F>>::new()
        .typed_post(create_transcription::handler::<F>)
        .layer(DefaultBodyLimit::max(MAX_AUDIO_BYTES + MULTIPART_OVERHEAD))
        .with_state(TranscriptionsState::new(usecase.clone(), factory));
    let documents = Router::<DocumentsState<R>>::new()
        .typed_post(read_document::handler::<R>)
        .layer(DefaultBodyLimit::max(MAX_PDF_BYTES + MULTIPART_OVERHEAD))
        .with_state(DocumentsState::new(usecase, readers));
    transcriptions.merge(documents)
}
