use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct InboundMetrics {
    pub active_connections: AtomicU64,
    pub accepted_messages: AtomicU64,
    pub rejected_messages: AtomicU64,
    pub temporary_failures: AtomicU64,
    pub message_bytes: AtomicU64,
}

impl InboundMetrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            accepted_messages: self.accepted_messages.load(Ordering::Relaxed),
            rejected_messages: self.rejected_messages.load(Ordering::Relaxed),
            temporary_failures: self.temporary_failures.load(Ordering::Relaxed),
            message_bytes: self.message_bytes.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetricsSnapshot {
    pub active_connections: u64,
    pub accepted_messages: u64,
    pub rejected_messages: u64,
    pub temporary_failures: u64,
    pub message_bytes: u64,
}
