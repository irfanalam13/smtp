use std::{net::SocketAddr, time::Duration};

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TlsPolicy {
    Opportunistic,
    Required,
}

#[derive(Clone, Debug)]
pub struct DeliveryConfig {
    pub enabled: bool,
    pub test_endpoint: Option<SocketAddr>,
    pub ehlo_hostname: String,
    pub dns_timeout: Duration,
    pub connect_timeout: Duration,
    pub tls_timeout: Duration,
    pub command_timeout: Duration,
    pub overall_timeout: Duration,
    pub max_concurrent: usize,
    pub max_message_bytes: usize,
    pub max_response_bytes: usize,
    pub max_mx_candidates: usize,
    pub max_fallback_attempts: usize,
    pub tls_policy: TlsPolicy,
    pub require_public_destinations: bool,
    pub allow_loopback_test_destinations: bool,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            test_endpoint: None,
            ehlo_hostname: "smtp-platform.local".into(),
            dns_timeout: Duration::from_secs(3),
            connect_timeout: Duration::from_secs(5),
            tls_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(10),
            overall_timeout: Duration::from_secs(30),
            max_concurrent: 16,
            max_message_bytes: 10 * 1024 * 1024,
            max_response_bytes: 64 * 1024,
            max_mx_candidates: 5,
            max_fallback_attempts: 3,
            tls_policy: TlsPolicy::Opportunistic,
            require_public_destinations: true,
            allow_loopback_test_destinations: false,
        }
    }
}

impl DeliveryConfig {
    pub fn from_env() -> Result<Self, DeliveryConfigError> {
        let enabled = std::env::var("SMTP_DELIVERY_ENABLED")
            .map(|value| value == "true")
            .unwrap_or(false);
        let config = Self {
            enabled,
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), DeliveryConfigError> {
        if self.ehlo_hostname.is_empty()
            || self.max_concurrent == 0
            || self.max_message_bytes == 0
            || self.max_response_bytes == 0
            || self.max_mx_candidates == 0
            || self.max_fallback_attempts == 0
            || self.overall_timeout.is_zero()
            || self.connect_timeout > self.overall_timeout
        {
            return Err(DeliveryConfigError::Invalid);
        }
        if self.enabled
            && !self.require_public_destinations
            && !self.allow_loopback_test_destinations
        {
            return Err(DeliveryConfigError::UnsafeDestinationPolicy);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DeliveryConfigError {
    #[error("invalid delivery configuration")]
    Invalid,
    #[error("real delivery requires public destination enforcement or explicit loopback test mode")]
    UnsafeDestinationPolicy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_delivery_is_disabled_by_default() {
        assert!(!DeliveryConfig::default().enabled);
        assert!(DeliveryConfig::default().validate().is_ok());
    }
}
