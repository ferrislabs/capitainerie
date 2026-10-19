use std::{sync::Arc, time::Duration};

use capitainerie_core::CapitainerieUseCase;

pub const TURN_DEADLINE: Duration = Duration::from_secs(8 * 60);

pub struct ConversationsState<R, H, X> {
    pub usecase: CapitainerieUseCase,
    pub runtime: Arc<R>,
    pub code_host: Arc<H>,
    /// `None` when the API has no exchange credentials: the Captain then
    /// cannot act for anyone, and says so.
    pub exchanger: Option<Arc<X>>,
    /// How long one turn may run; below the lease, so a turn ends before
    /// another may take its place.
    pub deadline: Duration,
}

impl<R, H, X> ConversationsState<R, H, X> {
    pub fn new(
        usecase: CapitainerieUseCase,
        runtime: R,
        code_host: Arc<H>,
        exchanger: Option<X>,
    ) -> Self {
        Self {
            usecase,
            runtime: Arc::new(runtime),
            code_host,
            exchanger: exchanger.map(Arc::new),
            deadline: TURN_DEADLINE,
        }
    }

    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }
}

impl<R, H, X> Clone for ConversationsState<R, H, X> {
    fn clone(&self) -> Self {
        Self {
            usecase: self.usecase.clone(),
            runtime: Arc::clone(&self.runtime),
            code_host: Arc::clone(&self.code_host),
            exchanger: self.exchanger.as_ref().map(Arc::clone),
            deadline: self.deadline,
        }
    }
}
