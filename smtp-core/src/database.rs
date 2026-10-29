use std::{fmt, time::Duration};

use sqlx::{PgPool, postgres::PgPoolOptions};
use thiserror::Error;

#[derive(Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
    pub statement_timeout: Option<Duration>,
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DatabaseConfig")
            .field("database_url", &"REDACTED")
            .field("max_connections", &self.max_connections)
            .field("acquire_timeout", &self.acquire_timeout)
            .field("statement_timeout", &self.statement_timeout)
            .finish()
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: String::new(),
            max_connections: 8,
            acquire_timeout: Duration::from_secs(5),
            statement_timeout: Some(Duration::from_secs(30)),
        }
    }
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, DatabaseError> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| DatabaseError::Configuration("DATABASE_URL is required".into()))?;
        let config = Self {
            database_url,
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), DatabaseError> {
        if self.database_url.trim().is_empty() {
            return Err(DatabaseError::Configuration(
                "DATABASE_URL must not be empty".into(),
            ));
        }
        if self.max_connections == 0 {
            return Err(DatabaseError::Configuration(
                "database pool size must be greater than zero".into(),
            ));
        }
        if self.acquire_timeout.is_zero() {
            return Err(DatabaseError::Configuration(
                "database acquire timeout must be greater than zero".into(),
            ));
        }
        if self
            .statement_timeout
            .is_some_and(|timeout| timeout.is_zero())
        {
            return Err(DatabaseError::Configuration(
                "database statement timeout must be greater than zero".into(),
            ));
        }
        Ok(())
    }

    pub async fn connect(&self) -> Result<PgPool, DatabaseError> {
        self.validate()?;
        let statement_timeout = self
            .statement_timeout
            .map(|timeout| timeout.as_millis().to_string());
        let options = PgPoolOptions::new()
            .max_connections(self.max_connections)
            .acquire_timeout(self.acquire_timeout);
        let options = if let Some(statement_timeout) = statement_timeout {
            options.after_connect(move |connection, _| {
                let statement_timeout = statement_timeout.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('statement_timeout', $1, false)")
                        .bind(statement_timeout)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
        } else {
            options
        };
        Ok(options.connect(&self.database_url).await?)
    }

    pub async fn check_ready(pool: &PgPool) -> Result<(), DatabaseError> {
        sqlx::query("SELECT 1").execute(pool).await?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("invalid database configuration: {0}")]
    Configuration(String),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_redacts_connection_string() {
        let config = DatabaseConfig {
            database_url: "postgres://user:password@example.test/db".into(),
            ..DatabaseConfig::default()
        };
        let output = format!("{config:?}");
        assert!(output.contains("REDACTED"));
        assert!(!output.contains("password"));
    }

    #[test]
    fn invalid_pool_settings_are_rejected() {
        let config = DatabaseConfig {
            max_connections: 0,
            ..DatabaseConfig::default()
        };
        assert!(config.validate().is_err());
    }
}
