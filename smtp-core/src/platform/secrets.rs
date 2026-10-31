use std::{collections::HashMap, sync::Arc};

use thiserror::Error;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(value: impl Into<String>) -> Result<Self, SecretError> {
        let value = value.into();
        if value.trim().is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_-.{}".contains(&byte))
        {
            return Err(SecretError::InvalidKey);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(Arc<str>);

impl SecretValue {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretValue(REDACTED)")
    }
}

pub trait SecretProvider: Send + Sync {
    fn get(&self, key: &SecretKey) -> Result<SecretValue, SecretError>;
}

#[derive(Clone, Default)]
pub struct EnvSecretProvider {
    values: HashMap<String, String>,
}

impl std::fmt::Debug for EnvSecretProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EnvSecretProvider")
            .field("keys", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl EnvSecretProvider {
    pub fn from_env<I>(values: I) -> Result<Self, SecretError>
    where
        I: IntoIterator<Item = SecretKey>,
    {
        let mut provider = Self::default();
        for key in values {
            let value = std::env::var(key.as_str()).map_err(|error| match error {
                std::env::VarError::NotPresent => SecretError::Missing(key.as_str().to_owned()),
                std::env::VarError::NotUnicode(_) => {
                    SecretError::InvalidValue(key.as_str().to_owned())
                }
            })?;
            provider.values.insert(key.0, value);
        }
        Ok(provider)
    }

    pub fn from_values(values: HashMap<String, String>) -> Self {
        Self { values }
    }
}

impl SecretProvider for EnvSecretProvider {
    fn get(&self, key: &SecretKey) -> Result<SecretValue, SecretError> {
        self.values
            .get(key.as_str())
            .map(|value| SecretValue(Arc::from(value.as_str())))
            .ok_or_else(|| SecretError::Missing(key.as_str().to_owned()))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretError {
    #[error("invalid secret key")]
    InvalidKey,
    #[error("required secret is missing: {0}")]
    Missing(String),
    #[error("secret value is not valid UTF-8 for key: {0}")]
    InvalidValue(String),
    #[error("secret provider failed")]
    Provider,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_output_is_redacted() {
        let provider = EnvSecretProvider::from_values(HashMap::from([(
            "DATABASE_PASSWORD".to_owned(),
            "do-not-log-me".to_owned(),
        )]));
        let secret = provider
            .get(&SecretKey::new("DATABASE_PASSWORD").unwrap())
            .unwrap();
        assert_eq!(format!("{secret:?}"), "SecretValue(REDACTED)");
        assert_eq!(secret.expose(), "do-not-log-me");
    }

    #[test]
    fn invalid_secret_keys_are_rejected() {
        assert!(SecretKey::new("bad key").is_err());
    }
}
