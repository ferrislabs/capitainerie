use std::sync::Arc;

pub struct AnalyticsState<A> {
    pub analytics: Arc<A>,
}

impl<A> AnalyticsState<A> {
    pub fn new(analytics: A) -> Self {
        Self {
            analytics: Arc::new(analytics),
        }
    }
}

impl<A> Clone for AnalyticsState<A> {
    fn clone(&self) -> Self {
        Self {
            analytics: Arc::clone(&self.analytics),
        }
    }
}
