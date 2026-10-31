use chrono::{DateTime, Utc};
use shared_types::{CANONICAL_MESSAGE_SCHEMA_VERSION, CanonicalMessage};
use uuid::Uuid;

use crate::error::QueueError;

pub type MessageId = Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueueState {
    Queued,
    Processing,
    RetryScheduled,
    SimulatedSucceeded,
    TerminalFailed,
    AcceptedByRemote,
    PartiallyAccepted,
    PermanentlyFailed,
    Cancelled,
    Suppressed,
    DeliveryUncertain,
    DeliveryProcessing,
}

impl QueueState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Processing => "processing",
            Self::RetryScheduled => "retry_scheduled",
            Self::SimulatedSucceeded => "simulated_succeeded",
            Self::TerminalFailed => "terminal_failed",
            Self::AcceptedByRemote => "accepted_by_remote",
            Self::PartiallyAccepted => "partially_accepted",
            Self::PermanentlyFailed => "permanently_failed",
            Self::Cancelled => "cancelled",
            Self::Suppressed => "suppressed",
            Self::DeliveryUncertain => "delivery_uncertain",
            Self::DeliveryProcessing => "delivery_processing",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::SimulatedSucceeded
                | Self::TerminalFailed
                | Self::AcceptedByRemote
                | Self::PartiallyAccepted
                | Self::PermanentlyFailed
                | Self::Cancelled
                | Self::Suppressed
        )
    }

    pub fn is_active(&self) -> bool {
        !self.is_terminal()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailureCategory {
    Temporary,
    Permanent,
    LeaseLost,
    Cancelled,
    Internal,
}

impl FailureCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Temporary => "temporary",
            Self::Permanent => "permanent",
            Self::LeaseLost => "lease_lost",
            Self::Cancelled => "cancelled",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug)]
pub struct EnqueueRequest<'a> {
    pub canonical_message: &'a CanonicalMessage,
    pub idempotency_key: String,
    pub idempotency_scope: String,
}

impl<'a> EnqueueRequest<'a> {
    pub fn new(
        canonical_message: &'a CanonicalMessage,
        idempotency_key: String,
        idempotency_scope: String,
    ) -> Self {
        Self {
            canonical_message,
            idempotency_key,
            idempotency_scope,
        }
    }

    pub fn validate(&self, config: &crate::QueueConfig) -> Result<(), QueueError> {
        let envelope = self.canonical_message.envelope();
        if self.canonical_message.schema_version() != CANONICAL_MESSAGE_SCHEMA_VERSION
            || !valid_reverse_path(&envelope.sender)
            || envelope.recipients.is_empty()
            || envelope.recipients.len() > config.max_recipients
            || self.canonical_message.raw_message().len() > config.max_message_bytes
            || self.idempotency_key.is_empty()
            || self.idempotency_scope.is_empty()
            || envelope
                .recipients
                .iter()
                .any(|recipient| !valid_forward_path(recipient))
        {
            return Err(QueueError::InvalidInput(
                "sender, recipients, body, or idempotency data is invalid".into(),
            ));
        }
        Ok(())
    }
}

fn valid_reverse_path(path: &str) -> bool {
    path.is_empty() || valid_forward_path(path)
}

fn valid_forward_path(path: &str) -> bool {
    if path.is_empty() || path.len() > 254 || !path.is_ascii() {
        return false;
    }
    let Some((local, domain)) = path.rsplit_once('@') else {
        return false;
    };
    valid_dot_atom(local) && valid_domain(domain)
}

fn valid_dot_atom(local: &str) -> bool {
    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'/'
                        | b'='
                        | b'?'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'{'
                        | b'|'
                        | b'}'
                        | b'~'
                        | b'.'
                )
        })
}

fn valid_domain(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    let domain = domain.strip_suffix('.').unwrap_or(domain);
    !domain.is_empty()
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnqueueResult {
    pub message_id: MessageId,
    pub created: bool,
}

#[derive(Clone, Debug)]
pub struct ClaimedJob {
    pub message_id: MessageId,
    pub sender: String,
    pub recipients: Vec<String>,
    pub body: Vec<u8>,
    pub attempt_count: u32,
    pub lease_token: Uuid,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct ClaimedDeliveryBatch {
    pub message_id: MessageId,
    pub envelope_sender: String,
    pub recipients: Vec<String>,
    pub destination_domain: String,
    pub raw_message: Vec<u8>,
    pub schema_version: u16,
    pub created_at: DateTime<Utc>,
    pub attempt_recipients: Vec<ClaimedRecipient>,
    pub lease_token: Uuid,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimedRecipient {
    pub delivery_id: i64,
    pub address: String,
    pub attempt_number: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipientDeliveryState {
    AcceptedByRemote,
    TemporaryFailure,
    PermanentFailure,
    Uncertain,
    Suppressed,
}

impl RecipientDeliveryState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AcceptedByRemote => "accepted_by_remote",
            Self::TemporaryFailure => "temporary_failure",
            Self::PermanentFailure => "permanent_failure",
            Self::Uncertain => "uncertain",
            Self::Suppressed => "suppressed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipientDeliveryResult {
    pub delivery_id: i64,
    pub state: RecipientDeliveryState,
    pub smtp_code: Option<u16>,
    pub diagnostic: Option<String>,
    pub remote_host: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadLetterItem {
    pub message_id: MessageId,
    pub envelope_sender: String,
    pub state: QueueState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub recipient_deliveries: Vec<DeadLetterRecipient>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadLetterRecipient {
    pub delivery_id: i64,
    pub recipient: String,
    pub destination_domain: String,
    pub state: String,
    pub attempt_count: u32,
    pub last_smtp_code: Option<u16>,
    pub last_diagnostic: Option<String>,
    pub last_remote_host: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{valid_forward_path, valid_reverse_path};
    use shared_types::{CanonicalMessage, MessageEnvelope, MimeLimits};

    #[test]
    fn accepts_bounded_smtp_paths() {
        assert!(valid_forward_path("sender@example.test"));
        assert!(valid_reverse_path(""));
        assert!(valid_reverse_path("sender@example.test"));
    }

    #[test]
    fn rejects_ambiguous_or_unsafe_smtp_paths() {
        assert!(!valid_forward_path("sender"));
        assert!(!valid_forward_path("sender @example.test"));
        assert!(!valid_forward_path("two..dots@example.test"));
        assert!(!valid_forward_path(
            "sender@example.test\r\nX-Injected: yes"
        ));
        assert!(!valid_forward_path("é@example.test"));
        assert!(!valid_reverse_path("<>"));
    }

    #[test]
    fn canonical_request_keeps_envelope_independent_from_message_headers() {
        let canonical = CanonicalMessage::parse(
            MessageEnvelope {
                sender: "bounce@example.test".into(),
                recipients: vec!["recipient@example.test".into()],
            },
            b"From: header@example.test\r\nTo: header-recipient@example.test\r\n\r\nbody".to_vec(),
            MimeLimits::default(),
        )
        .unwrap();
        let request = super::EnqueueRequest::new(&canonical, "key".into(), "scope".into());
        assert_eq!(
            request.canonical_message.envelope().sender,
            "bounce@example.test"
        );
        assert_eq!(
            request.canonical_message.parsed().from,
            vec!["header@example.test"]
        );
    }
}
