use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorClass {
    Permanent,
    Transient,
    Unknown,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("configuration error: {0}")]
    Configuration(#[from] crate::config::ConfigError),
    #[error("secret provider error: {0}")]
    Secret(#[from] crate::secrets::SecretError),
    #[error("event serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid event: {0}")]
    InvalidEvent(&'static str),
}

impl CoreError {
    pub fn class(&self) -> ErrorClass {
        match self {
            Self::Configuration(_) | Self::InvalidEvent(_) => ErrorClass::Permanent,
            Self::Secret(crate::secrets::SecretError::Missing(_))
            | Self::Secret(crate::secrets::SecretError::InvalidValue(_)) => ErrorClass::Permanent,
            Self::Secret(crate::secrets::SecretError::Provider) => ErrorClass::Transient,
            Self::Secret(crate::secrets::SecretError::InvalidKey) | Self::Serialization(_) => {
                ErrorClass::Unknown
            }
        }
    }
}
