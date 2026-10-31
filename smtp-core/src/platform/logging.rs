use std::sync::OnceLock;

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Environment;

static INITIALIZED: OnceLock<()> = OnceLock::new();

pub fn init(
    service_name: &str,
    environment: Environment,
    filter: &str,
) -> Result<(), LoggingError> {
    if service_name.trim().is_empty() || filter.trim().is_empty() {
        return Err(LoggingError::InvalidConfiguration);
    }
    if INITIALIZED.get().is_some() {
        return Err(LoggingError::AlreadyInitialized);
    }
    let env_filter = EnvFilter::try_new(filter).map_err(|_| LoggingError::InvalidFilter)?;
    let result = if environment == Environment::Development || environment == Environment::Test {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().compact())
            .try_init()
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .try_init()
    };
    result.map_err(|_| LoggingError::AlreadyInitialized)?;
    let _ = INITIALIZED.set(());
    tracing::info!(service = service_name, environment = ?environment, "logging initialized");
    Ok(())
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum LoggingError {
    #[error("logging configuration is empty")]
    InvalidConfiguration,
    #[error("log filter is invalid")]
    InvalidFilter,
    #[error("logging has already been initialized")]
    AlreadyInitialized,
}
