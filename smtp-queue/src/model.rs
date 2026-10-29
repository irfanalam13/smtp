use chrono::{DateTime, Utc};
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
}

impl QueueState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Processing => "processing",
            Self::RetryScheduled => "retry_scheduled",
            Self::SimulatedSucceeded => "simulated_succeeded",
            Self::TerminalFailed => "terminal_failed",
        }
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

#[derive(Clone, Debug)]
pub struct EnqueueRequest {
    pub sender: String,
    pub recipients: Vec<String>,
    pub body: Vec<u8>,
    pub idempotency_key: String,
    pub idempotency_scope: String,
}

impl EnqueueRequest {
    pub fn validate(&self, config: &crate::QueueConfig) -> Result<(), QueueError> {
        if !valid_reverse_path(&self.sender)
            || self.recipients.is_empty()
            || self.recipients.len() > config.max_recipients
            || self.body.len() > config.max_message_bytes
            || self.idempotency_key.is_empty()
            || self.idempotency_scope.is_empty()
            || self
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
    if path.is_empty() || path.len() > 320 || !path.is_ascii() {
        return false;
    }
    let Some((local, domain)) = path.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !local
            .chars()
            .chain(domain.chars())
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
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

#[cfg(test)]
mod tests {
    use super::{valid_forward_path, valid_reverse_path};

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
        assert!(!valid_forward_path(
            "sender@example.test\r\nX-Injected: yes"
        ));
        assert!(!valid_forward_path("é@example.test"));
        assert!(!valid_reverse_path("<>"));
    }
}
