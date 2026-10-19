use std::sync::Arc;

use capitainerie_core::CapitainerieUseCase;

pub struct TranscriptionsState<F> {
    pub usecase: CapitainerieUseCase,
    pub factory: Arc<F>,
}

impl<F> TranscriptionsState<F> {
    pub fn new(usecase: CapitainerieUseCase, factory: F) -> Self {
        Self {
            usecase,
            factory: Arc::new(factory),
        }
    }
}

impl<F> Clone for TranscriptionsState<F> {
    fn clone(&self) -> Self {
        Self {
            usecase: self.usecase.clone(),
            factory: Arc::clone(&self.factory),
        }
    }
}
