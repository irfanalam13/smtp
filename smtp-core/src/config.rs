use std::{net::SocketAddr, str::FromStr, time::Duration};

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Environment {
    Development,
    Test,
    Staging,
    Production,
}

impl Environment {
    pub fn is_production(self) -> bool {
        self == Self::Production
    }
}

impl FromStr for Environment {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "test" | "testing" => Ok(Self::Test),
            "staging" | "stage" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(ConfigError::InvalidValue {
                field: "APP_ENVIRONMENT",
                detail: "expected development, test, staging, or production",
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlatformConfig {
    pub service_name: String,
    pub environment: Environment,
    pub bind_address: SocketAddr,
    pub log_filter: String,
    pub shutdown_timeout: Duration,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            service_name: "smtp-platform-service".to_owned(),
            environment: Environment::Development,
            bind_address: SocketAddr::from(([127, 0, 0, 1], 8080)),
            log_filter: "info".to_owned(),
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

impl PlatformConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        if let Some(value) = env_value("SERVICE_NAME")? {
            config.service_name = value;
        }
        if let Some(value) = env_value("APP_ENVIRONMENT")? {
            config.environment = value.parse()?;
        }
        if let Some(value) = env_value("BIND_ADDRESS")? {
            config.bind_address = value.parse().map_err(|_| ConfigError::InvalidValue {
                field: "BIND_ADDRESS",
                detail: "expected a valid socket address",
            })?;
        }
        if let Some(value) = env_value("LOG_FILTER")? {
            config.log_filter = value;
        }
        if let Some(value) = env_value("SHUTDOWN_TIMEOUT_MS")? {
            let millis = value
                .parse::<u64>()
                .map_err(|_| ConfigError::InvalidValue {
                    field: "SHUTDOWN_TIMEOUT_MS",
                    detail: "expected a non-negative integer",
                })?;
            config.shutdown_timeout = Duration::from_millis(millis);
        }
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.service_name.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "SERVICE_NAME",
                detail: "must not be empty",
            });
        }
        if self.log_filter.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "LOG_FILTER",
                detail: "must not be empty",
            });
        }
        if self.shutdown_timeout.is_zero() || self.shutdown_timeout > Duration::from_secs(300) {
            return Err(ConfigError::InvalidValue {
                field: "SHUTDOWN_TIMEOUT_MS",
                detail: "must be greater than zero and no more than 300 seconds",
            });
        }
        if self.environment.is_production() && self.bind_address.ip().is_loopback() {
            return Err(ConfigError::InvalidValue {
                field: "BIND_ADDRESS",
                detail: "production services must not bind to loopback",
            });
        }
        Ok(())
    }
}

fn env_value(name: &'static str) -> Result<Option<String>, ConfigError> {
    std::env::var(name).map(Some).or_else(|error| {
        if error == std::env::VarError::NotPresent {
            Ok(None)
        } else {
            Err(ConfigError::InvalidValue {
                field: name,
                detail: "must contain valid UTF-8",
            })
        }
    })
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("invalid configuration field {field}: {detail}")]
    InvalidValue {
        field: &'static str,
        detail: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_for_development() {
        assert!(PlatformConfig::default().validate().is_ok());
    }

    #[test]
    fn production_loopback_is_rejected() {
        let config = PlatformConfig {
            environment: Environment::Production,
            ..PlatformConfig::default()
        };
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidValue {
                field: "BIND_ADDRESS",
                detail: "production services must not bind to loopback"
            })
        );
    }

    #[test]
    fn environment_parser_rejects_unknown_values() {
        assert!("unknown".parse::<Environment>().is_err());
    }
}
