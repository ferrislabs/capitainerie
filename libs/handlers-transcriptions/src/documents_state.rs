use std::sync::Arc;

use capitainerie_core::CapitainerieUseCase;

pub struct DocumentsState<R> {
    pub usecase: CapitainerieUseCase,
    pub readers: Arc<R>,
}

impl<R> DocumentsState<R> {
    pub fn new(usecase: CapitainerieUseCase, readers: R) -> Self {
        Self {
            usecase,
            readers: Arc::new(readers),
        }
    }
}

impl<R> Clone for DocumentsState<R> {
    fn clone(&self) -> Self {
        Self {
            usecase: self.usecase.clone(),
            readers: Arc::clone(&self.readers),
        }
    }
}
