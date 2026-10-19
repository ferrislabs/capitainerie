use std::sync::Arc;

use capitainerie_core::CapitainerieUseCase;
use tokio::sync::Semaphore;

use crate::MAX_CONCURRENT_UPLOADS;

pub struct DocumentsState<R> {
    pub usecase: CapitainerieUseCase,
    pub readers: Arc<R>,
    pub permits: Arc<Semaphore>,
}

impl<R> DocumentsState<R> {
    pub fn new(usecase: CapitainerieUseCase, readers: R) -> Self {
        Self {
            usecase,
            readers: Arc::new(readers),
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_UPLOADS)),
        }
    }
}

impl<R> Clone for DocumentsState<R> {
    fn clone(&self) -> Self {
        Self {
            usecase: self.usecase.clone(),
            readers: Arc::clone(&self.readers),
            permits: Arc::clone(&self.permits),
        }
    }
}
