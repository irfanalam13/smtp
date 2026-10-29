use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const MAX_ID_LENGTH: usize = 128;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ContextError> {
                let value = value.into();
                if value.is_empty()
                    || value.len() > MAX_ID_LENGTH
                    || value
                        .bytes()
                        .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
                {
                    return Err(ContextError::InvalidIdentifier);
                }
                Ok(Self(value))
            }

            pub fn generate() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

identifier!(CorrelationId);
identifier!(RequestId);
identifier!(TraceId);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationContext {
    pub correlation_id: Option<CorrelationId>,
    pub request_id: Option<RequestId>,
    pub trace_id: Option<TraceId>,
}

impl OperationContext {
    pub fn with_generated_ids() -> Self {
        Self {
            correlation_id: Some(CorrelationId::generate()),
            request_id: Some(RequestId::generate()),
            trace_id: Some(TraceId::generate()),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContextError {
    #[error("context identifier is empty, too long, or contains whitespace/control characters")]
    InvalidIdentifier,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_distinct_types() {
        let context = OperationContext::with_generated_ids();
        assert_ne!(
            context.correlation_id.as_ref().unwrap().as_str(),
            context.request_id.as_ref().unwrap().as_str()
        );
    }

    #[test]
    fn untrusted_identifier_input_is_bounded() {
        assert!(CorrelationId::new("bad\nvalue").is_err());
        assert!(RequestId::new("x".repeat(MAX_ID_LENGTH + 1)).is_err());
    }
}
