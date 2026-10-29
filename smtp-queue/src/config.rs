use std::{str::FromStr, time::Duration};

use crate::error::QueueError;

#[derive(Clone, Debug)]
pub struct QueueConfig {
    pub database_url: String,
    pub max_pool_connections: u32,
    pub acquire_timeout: Duration,
    pub max_message_bytes: usize,
    pub max_recipients: usize,
    pub initial_retry_delay: Duration,
    pub max_retry_delay: Duration,
    pub max_attempts: u32,
    pub lease_duration: Duration,
    pub jitter: Duration,
}

impl QueueConfig {
    pub fn from_env() -> Result<Self, QueueError> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| QueueError::Configuration("DATABASE_URL is required".into()))?;
        let config = Self {
            database_url,
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), QueueError> {
        if self.database_url.is_empty()
            || self.max_pool_connections == 0
            || self.max_message_bytes == 0
            || self.max_recipients == 0
            || self.max_attempts == 0
            || self.lease_duration.is_zero()
            || self.initial_retry_delay > self.max_retry_delay
        {
            return Err(QueueError::Configuration(
                "invalid queue configuration".into(),
            ));
        }
        Ok(())
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            database_url: String::new(),
            max_pool_connections: 8,
            acquire_timeout: Duration::from_secs(5),
            max_message_bytes: 10 * 1024 * 1024,
            max_recipients: 100,
            initial_retry_delay: Duration::from_secs(30),
            max_retry_delay: Duration::from_secs(3600),
            max_attempts: 5,
            lease_duration: Duration::from_secs(60),
            jitter: Duration::from_secs(5),
        }
    }
}

pub fn duration_from_millis(value: u64) -> Result<Duration, QueueError> {
    Duration::from_millis(value)
        .checked_add(Duration::ZERO)
        .ok_or_else(|| QueueError::Configuration("duration overflow".into()))
}

impl FromStr for QueueConfig {
    type Err = QueueError;
    fn from_str(database_url: &str) -> Result<Self, Self::Err> {
        let config = Self {
            database_url: database_url.to_string(),
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_or_incomplete_configuration() {
        assert!(QueueConfig::default().validate().is_err());
        assert!(
            QueueConfig {
                database_url: "postgres://test".into(),
                max_attempts: 0,
                ..QueueConfig::default()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn accepts_valid_configuration() {
        assert!("postgres://test".parse::<QueueConfig>().is_ok());
    }
}
