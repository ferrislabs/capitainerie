use std::sync::Arc;

use tokio::sync::{broadcast, watch};

use crate::wire::Notice;

struct Inner {
    notices: broadcast::Sender<Arc<Notice>>,
    degraded: watch::Sender<bool>,
}

#[derive(Clone)]
pub struct Hub {
    inner: Arc<Inner>,
}

impl Hub {
    /// Starts degraded: nothing feeds the hub until the consumer is connected.
    pub fn new(capacity: usize) -> Self {
        let (notices, _) = broadcast::channel(capacity.max(1));
        let (degraded, _) = watch::channel(true);
        Self {
            inner: Arc::new(Inner { notices, degraded }),
        }
    }

    pub fn publish(&self, notice: Notice) {
        let _ = self.inner.notices.send(Arc::new(notice));
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Notice>> {
        self.inner.notices.subscribe()
    }

    pub fn watch_degraded(&self) -> watch::Receiver<bool> {
        self.inner.degraded.subscribe()
    }

    pub fn set_degraded(&self, degraded: bool) {
        self.inner.degraded.send_if_modified(|current| {
            let changed = *current != degraded;
            *current = degraded;
            changed
        });
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::wire::ServerMessage;

    fn notice() -> Notice {
        Notice {
            organization_id: Uuid::now_v7(),
            message: ServerMessage::Resync,
        }
    }

    #[tokio::test]
    async fn subscribers_receive_what_is_published_after_they_subscribe() {
        let hub = Hub::new(8);
        hub.publish(notice());
        let mut rx = hub.subscribe();
        let sent = notice();
        hub.publish(sent.clone());
        assert_eq!(*rx.recv().await.unwrap(), sent);
    }

    #[tokio::test]
    async fn degraded_starts_true_and_notifies_only_on_change() {
        let hub = Hub::new(8);
        let mut rx = hub.watch_degraded();
        assert!(*rx.borrow_and_update());
        hub.set_degraded(true);
        assert!(!rx.has_changed().unwrap());
        hub.set_degraded(false);
        assert!(rx.has_changed().unwrap());
        assert!(!*rx.borrow_and_update());
    }

    #[tokio::test]
    async fn a_slow_subscriber_lags_instead_of_blocking_publishers() {
        let hub = Hub::new(2);
        let mut rx = hub.subscribe();
        for _ in 0..5 {
            hub.publish(notice());
        }
        assert!(matches!(
            rx.recv().await,
            Err(broadcast::error::RecvError::Lagged(_))
        ));
    }
}
