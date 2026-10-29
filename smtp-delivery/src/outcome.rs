use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryOutcome {
    DeliveredAccepted,
    TemporaryFailure {
        code: Option<u16>,
        category: &'static str,
    },
    PermanentFailure {
        code: Option<u16>,
        category: &'static str,
    },
    SecurityFailure {
        category: &'static str,
    },
    UnsafeDestination,
    Cancelled,
    LeaseLost,
    InternalFailure,
}

impl DeliveryOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::DeliveredAccepted)
    }
}

#[derive(Debug, Error)]
pub enum DeliveryError {
    #[error("DNS resolver failure")]
    Dns,
    #[error("no usable MX candidate")]
    NoCandidate,
    #[error("unsafe destination")]
    UnsafeDestination,
    #[error("connection timeout")]
    ConnectTimeout,
    #[error("SMTP response error: {0}")]
    Protocol(String),
    #[error("TLS negotiation or validation failed")]
    Tls,
    #[error("message exceeds delivery limit")]
    MessageTooLarge,
    #[error("delivery disabled")]
    Disabled,
    #[error("ambiguous final SMTP response")]
    AmbiguousFinalResponse,
    #[error("internal delivery failure")]
    InternalFailure,
    #[error("permanent delivery failure: {category}")]
    PermanentFailure {
        code: Option<u16>,
        category: &'static str,
    },
    #[error("delivery security failure: {category}")]
    SecurityFailure { category: &'static str },
}

pub fn classify_status(code: u16) -> DeliveryOutcome {
    match code {
        200..=399 => DeliveryOutcome::DeliveredAccepted,
        400..=499 => DeliveryOutcome::TemporaryFailure {
            code: Some(code),
            category: "temporary_smtp",
        },
        _ => DeliveryOutcome::PermanentFailure {
            code: Some(code),
            category: "permanent_smtp",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_smtp_status_classes() {
        assert!(classify_status(250).is_success());
        assert!(matches!(
            classify_status(421),
            DeliveryOutcome::TemporaryFailure { .. }
        ));
        assert!(matches!(
            classify_status(550),
            DeliveryOutcome::PermanentFailure { .. }
        ));
    }
}
