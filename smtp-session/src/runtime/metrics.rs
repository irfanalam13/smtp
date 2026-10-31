use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct InboundMetrics {
    pub active_connections: AtomicU64,
    pub accepted_messages: AtomicU64,
    pub rejected_messages: AtomicU64,
    pub temporary_failures: AtomicU64,
    pub inbound_rate_limited: AtomicU64,
    pub inbound_quota_rejections: AtomicU64,
    pub inbound_malware_rejections: AtomicU64,
    pub inbound_spam_rejections: AtomicU64,
    pub inbound_auth_rejections: AtomicU64,
    pub message_bytes: AtomicU64,
    pub authentication_attempts: AtomicU64,
    pub authentication_failures: AtomicU64,
    pub authentication_throttled: AtomicU64,
    pub authentication_backend_failures: AtomicU64,
    pub starttls_upgrades: AtomicU64,
    pub tls_handshake_failures: AtomicU64,
}

impl InboundMetrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            accepted_messages: self.accepted_messages.load(Ordering::Relaxed),
            rejected_messages: self.rejected_messages.load(Ordering::Relaxed),
            temporary_failures: self.temporary_failures.load(Ordering::Relaxed),
            inbound_rate_limited: self.inbound_rate_limited.load(Ordering::Relaxed),
            inbound_quota_rejections: self.inbound_quota_rejections.load(Ordering::Relaxed),
            inbound_malware_rejections: self.inbound_malware_rejections.load(Ordering::Relaxed),
            inbound_spam_rejections: self.inbound_spam_rejections.load(Ordering::Relaxed),
            inbound_auth_rejections: self.inbound_auth_rejections.load(Ordering::Relaxed),
            message_bytes: self.message_bytes.load(Ordering::Relaxed),
            authentication_attempts: self.authentication_attempts.load(Ordering::Relaxed),
            authentication_failures: self.authentication_failures.load(Ordering::Relaxed),
            authentication_throttled: self.authentication_throttled.load(Ordering::Relaxed),
            authentication_backend_failures: self
                .authentication_backend_failures
                .load(Ordering::Relaxed),
            starttls_upgrades: self.starttls_upgrades.load(Ordering::Relaxed),
            tls_handshake_failures: self.tls_handshake_failures.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetricsSnapshot {
    pub active_connections: u64,
    pub accepted_messages: u64,
    pub rejected_messages: u64,
    pub temporary_failures: u64,
    pub inbound_rate_limited: u64,
    pub inbound_quota_rejections: u64,
    pub inbound_malware_rejections: u64,
    pub inbound_spam_rejections: u64,
    pub inbound_auth_rejections: u64,
    pub message_bytes: u64,
    pub authentication_attempts: u64,
    pub authentication_failures: u64,
    pub authentication_throttled: u64,
    pub authentication_backend_failures: u64,
    pub starttls_upgrades: u64,
    pub tls_handshake_failures: u64,
}
