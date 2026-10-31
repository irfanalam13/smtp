use std::{net::SocketAddr, path::PathBuf, time::Duration};

use thiserror::Error;

#[derive(Clone, Eq, PartialEq)]
pub struct SmtpRelayConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub mode: SmtpRelayMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmtpRelayMode {
    AuthenticatedTls,
    LocalMailpit,
}

impl std::fmt::Debug for SmtpRelayConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SmtpRelayConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .field("from_address", &self.from_address)
            .field("mode", &self.mode)
            .finish()
    }
}

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
    pub allowed_recipient_domains: Vec<String>,
    pub dkim_key_dir: Option<PathBuf>,
    pub smtp_relay: Option<SmtpRelayConfig>,
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
            allowed_recipient_domains: Vec::new(),
            dkim_key_dir: None,
            smtp_relay: None,
        }
    }
}

impl DeliveryConfig {
    pub fn from_env() -> Result<Self, DeliveryConfigError> {
        let enabled = env_bool("SMTP_DELIVERY_ENABLED", false)?;
        let allow_loopback_test_destinations =
            env_bool("SMTP_DELIVERY_ALLOW_LOOPBACK_TEST_DESTINATIONS", false)?;
        let ehlo_hostname = env_string("SMTP_DELIVERY_EHLO_HOSTNAME", "smtp-platform.local")?;
        let max_concurrent = env_usize("SMTP_DELIVERY_MAX_CONCURRENT", 16)?;
        let max_message_bytes = env_usize("SMTP_DELIVERY_MAX_MESSAGE_BYTES", 10 * 1024 * 1024)?;
        let allowlist = env_string("SMTP_DELIVERY_ALLOWLIST", "")?;
        let dkim_key_dir = std::env::var_os("SMTP_DKIM_KEY_DIR").map(PathBuf::from);
        let smtp_relay = smtp_relay_from_env(
            env_string("APP_ENVIRONMENT", "")?
                .trim()
                .eq_ignore_ascii_case("development"),
        )?;
        let allowed_recipient_domains = allowlist
            .split(',')
            .filter(|domain| !domain.trim().is_empty())
            .map(|domain| domain.trim().trim_end_matches('.').to_ascii_lowercase())
            .collect::<Vec<_>>();
        let tls_policy = match env_string("SMTP_DELIVERY_TLS_POLICY", "opportunistic")?.as_str() {
            "opportunistic" => TlsPolicy::Opportunistic,
            "required" => TlsPolicy::Required,
            _ => {
                return Err(DeliveryConfigError::InvalidSetting(
                    "SMTP_DELIVERY_TLS_POLICY",
                ));
            }
        };
        let config = Self {
            enabled,
            ehlo_hostname,
            max_concurrent,
            max_message_bytes,
            tls_policy,
            allow_loopback_test_destinations,
            allowed_recipient_domains,
            dkim_key_dir,
            smtp_relay,
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), DeliveryConfigError> {
        if self.ehlo_hostname.is_empty()
            || !valid_domain(&self.ehlo_hostname)
            || self.max_concurrent == 0
            || self.max_concurrent > 1_024
            || self.max_message_bytes == 0
            || self.max_message_bytes > shared_types::MAX_CANONICAL_MESSAGE_BYTES
            || self.max_response_bytes == 0
            || self.max_mx_candidates == 0
            || self.max_fallback_attempts == 0
            || self.max_fallback_attempts > 100
            || self.overall_timeout.is_zero()
            || self.connect_timeout > self.overall_timeout
            || self.allowed_recipient_domains.len() > 1_000
            || self
                .allowed_recipient_domains
                .iter()
                .any(|domain| !valid_domain(domain))
        {
            return Err(DeliveryConfigError::Invalid);
        }
        if self.enabled && !self.require_public_destinations {
            return Err(DeliveryConfigError::UnsafeDestinationPolicy);
        }
        if self.enabled && self.allow_loopback_test_destinations && self.test_endpoint.is_none() {
            return Err(DeliveryConfigError::UnsafeDestinationPolicy);
        }
        if self.enabled && self.smtp_relay.is_none() && self.allowed_recipient_domains.is_empty() {
            return Err(DeliveryConfigError::MissingRecipientAllowlist);
        }
        if self.enabled
            && self.smtp_relay.is_none()
            && self.test_endpoint.is_none()
            && self
                .dkim_key_dir
                .as_ref()
                .is_none_or(|directory| !directory.is_dir())
        {
            return Err(DeliveryConfigError::MissingDkimKeyDir);
        }
        if let Some(relay) = &self.smtp_relay {
            let valid = match relay.mode {
                SmtpRelayMode::AuthenticatedTls => {
                    self.test_endpoint.is_none()
                        && valid_domain(&relay.host)
                        && matches!(relay.port, 465 | 587)
                        && !relay.username.is_empty()
                        && relay.username.len() <= 512
                        && !relay.password.is_empty()
                        && relay.password.len() <= 512
                        && relay.username.len() + relay.password.len() + 2 <= 360
                        && ![relay.username.as_str(), relay.password.as_str()]
                            .iter()
                            .any(|value| value.chars().any(char::is_control))
                }
                SmtpRelayMode::LocalMailpit => {
                    relay.host == "127.0.0.1"
                        && relay.port == 1025
                        && relay.username.is_empty()
                        && relay.password.is_empty()
                }
            };
            if !valid || !valid_relay_address(&relay.from_address) {
                return Err(DeliveryConfigError::InvalidRelay);
            }
        }
        Ok(())
    }
}

fn smtp_relay_from_env(
    allow_local_mailpit: bool,
) -> Result<Option<SmtpRelayConfig>, DeliveryConfigError> {
    smtp_relay_from_values(
        optional_env("SMTP_RELAY_HOST")?,
        optional_env("SMTP_RELAY_PORT")?,
        optional_env("SMTP_RELAY_USERNAME")?,
        optional_env("SMTP_RELAY_PASSWORD")?,
        optional_env("SMTP_RELAY_FROM")?,
        allow_local_mailpit,
    )
}

fn smtp_relay_from_values(
    host: Option<String>,
    port: Option<String>,
    username: Option<String>,
    password: Option<String>,
    from_address: Option<String>,
    allow_local_mailpit: bool,
) -> Result<Option<SmtpRelayConfig>, DeliveryConfigError> {
    let values = [&host, &port, &username, &password, &from_address];
    if values.iter().all(|value| value.is_none()) {
        return Ok(None);
    }
    if host.as_deref() == Some("127.0.0.1")
        && port.as_deref() == Some("1025")
        && username.is_none()
        && password.is_none()
        && from_address.is_some()
    {
        if !allow_local_mailpit {
            return Err(DeliveryConfigError::LocalMailpitDevelopmentOnly);
        }
        return Ok(Some(SmtpRelayConfig {
            host: "127.0.0.1".into(),
            port: 1025,
            username: String::new(),
            password: String::new(),
            from_address: from_address.ok_or(DeliveryConfigError::IncompleteRelay)?,
            mode: SmtpRelayMode::LocalMailpit,
        }));
    }
    if values.iter().any(|value| value.is_none()) {
        return Err(DeliveryConfigError::IncompleteRelay);
    }
    let host = host.ok_or(DeliveryConfigError::IncompleteRelay)?;
    let port = port
        .ok_or(DeliveryConfigError::IncompleteRelay)?
        .parse::<u16>()
        .map_err(|_| DeliveryConfigError::InvalidSetting("SMTP_RELAY_PORT"))?;
    let username = username.ok_or(DeliveryConfigError::IncompleteRelay)?;
    let password = password.ok_or(DeliveryConfigError::IncompleteRelay)?;
    let from_address = from_address.ok_or(DeliveryConfigError::IncompleteRelay)?;
    Ok(Some(SmtpRelayConfig {
        host,
        port,
        username,
        password,
        from_address,
        mode: SmtpRelayMode::AuthenticatedTls,
    }))
}

fn optional_env(name: &'static str) -> Result<Option<String>, DeliveryConfigError> {
    match std::env::var(name) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(DeliveryConfigError::InvalidSetting(name)),
    }
}

fn valid_relay_address(value: &str) -> bool {
    value.len() <= 320
        && value.is_ascii()
        && !value.chars().any(char::is_whitespace)
        && value.matches('@').count() == 1
        && value
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && valid_domain(domain))
}

fn valid_domain(domain: &str) -> bool {
    let domain = domain.strip_suffix('.').unwrap_or(domain);
    !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn env_string(name: &'static str, default: &str) -> Result<String, DeliveryConfigError> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(std::env::VarError::NotUnicode(_)) => Err(DeliveryConfigError::InvalidSetting(name)),
    }
}

fn env_bool(name: &'static str, default: bool) -> Result<bool, DeliveryConfigError> {
    match std::env::var(name) {
        Ok(value) if value.eq_ignore_ascii_case("true") => Ok(true),
        Ok(value) if value.eq_ignore_ascii_case("false") => Ok(false),
        Ok(_) => Err(DeliveryConfigError::InvalidSetting(name)),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => Err(DeliveryConfigError::InvalidSetting(name)),
    }
}

fn env_usize(name: &'static str, default: usize) -> Result<usize, DeliveryConfigError> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| DeliveryConfigError::InvalidSetting(name)),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => Err(DeliveryConfigError::InvalidSetting(name)),
    }
}

#[derive(Debug, Error)]
pub enum DeliveryConfigError {
    #[error("invalid delivery configuration")]
    Invalid,
    #[error("real delivery requires public destination enforcement or explicit loopback test mode")]
    UnsafeDestinationPolicy,
    #[error("enabled direct-MX delivery requires SMTP_DELIVERY_ALLOWLIST")]
    MissingRecipientAllowlist,
    #[error("direct-MX delivery requires SMTP_DKIM_KEY_DIR")]
    MissingDkimKeyDir,
    #[error("SMTP relay settings must be configured together")]
    IncompleteRelay,
    #[error("unauthenticated Mailpit delivery is available only in development")]
    LocalMailpitDevelopmentOnly,
    #[error("invalid SMTP relay configuration")]
    InvalidRelay,
    #[error("invalid delivery setting: {0}")]
    InvalidSetting(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_delivery_is_disabled_by_default() {
        assert!(!DeliveryConfig::default().enabled);
        assert!(DeliveryConfig::default().validate().is_ok());
    }

    #[test]
    fn relay_config_does_not_expose_password_in_debug_output() {
        let relay = SmtpRelayConfig {
            host: "smtp.example.test".into(),
            port: 587,
            username: "user".into(),
            password: "secret-password".into(),
            from_address: "sender@example.test".into(),
            mode: SmtpRelayMode::AuthenticatedTls,
        };
        let debug = format!("{relay:?}");
        assert!(!debug.contains("secret-password"));
        assert!(!debug.contains("username: \"user\""));
    }

    #[test]
    fn relay_delivery_does_not_require_direct_mx_allowlist_or_dkim_keys() {
        let config = DeliveryConfig {
            enabled: true,
            smtp_relay: Some(SmtpRelayConfig {
                host: "smtp.example.test".into(),
                port: 587,
                username: "user".into(),
                password: "secret".into(),
                from_address: "sender@example.test".into(),
                mode: SmtpRelayMode::AuthenticatedTls,
            }),
            ..DeliveryConfig::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn unauthenticated_mailpit_relay_is_development_only() {
        let from = Some("sender@example.test".to_owned());
        let relay = smtp_relay_from_values(
            Some("127.0.0.1".into()),
            Some("1025".into()),
            None,
            None,
            from.clone(),
            true,
        )
        .expect("development Mailpit config");
        assert_eq!(
            relay.as_ref().map(|config| config.mode),
            Some(SmtpRelayMode::LocalMailpit)
        );
        assert!(
            smtp_relay_from_values(
                Some("127.0.0.1".into()),
                Some("1025".into()),
                None,
                None,
                from.clone(),
                false,
            )
            .is_err()
        );
        assert!(
            smtp_relay_from_values(
                Some("127.0.0.1".into()),
                Some("1025".into()),
                Some("unexpected".into()),
                None,
                from,
                true,
            )
            .is_err()
        );
    }

    #[test]
    fn local_mailpit_relay_requires_exact_loopback_endpoint_and_no_auth() {
        let config = DeliveryConfig {
            enabled: true,
            smtp_relay: Some(SmtpRelayConfig {
                host: "localhost".into(),
                port: 1025,
                username: String::new(),
                password: String::new(),
                from_address: "sender@example.test".into(),
                mode: SmtpRelayMode::LocalMailpit,
            }),
            ..DeliveryConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(DeliveryConfigError::InvalidRelay)
        ));
    }

    #[test]
    fn enabled_delivery_cannot_disable_public_destination_checks() {
        let config = DeliveryConfig {
            enabled: true,
            require_public_destinations: false,
            allow_loopback_test_destinations: true,
            allowed_recipient_domains: vec!["example.test".into()],
            ..DeliveryConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(DeliveryConfigError::UnsafeDestinationPolicy)
        ));
    }

    #[test]
    fn enabled_delivery_requires_a_valid_recipient_allowlist() {
        assert!(matches!(
            DeliveryConfig {
                enabled: true,
                ..DeliveryConfig::default()
            }
            .validate(),
            Err(DeliveryConfigError::MissingRecipientAllowlist)
        ));
        assert!(
            DeliveryConfig {
                enabled: false,
                allowed_recipient_domains: vec!["bad..domain".into()],
                ..DeliveryConfig::default()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn real_external_delivery_requires_dkim_key_mount() {
        let config = DeliveryConfig {
            enabled: true,
            allowed_recipient_domains: vec!["example.test".into()],
            ..DeliveryConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(DeliveryConfigError::MissingDkimKeyDir)
        ));
    }

    #[test]
    fn message_limit_cannot_exceed_canonical_parser_capacity() {
        let config = DeliveryConfig {
            max_message_bytes: shared_types::MAX_CANONICAL_MESSAGE_BYTES + 1,
            ..DeliveryConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(DeliveryConfigError::Invalid)
        ));
    }

    #[test]
    fn enabled_delivery_rejects_loopback_without_a_fixture_endpoint() {
        assert!(matches!(
            DeliveryConfig {
                enabled: true,
                allow_loopback_test_destinations: true,
                allowed_recipient_domains: vec!["example.test".into()],
                ..DeliveryConfig::default()
            }
            .validate(),
            Err(DeliveryConfigError::UnsafeDestinationPolicy)
        ));
    }

    #[test]
    fn rejects_ehlo_hostnames_that_could_inject_commands() {
        assert!(
            DeliveryConfig {
                ehlo_hostname: "smtp.example.test\r\nMAIL FROM:<evil@example.test>".into(),
                ..DeliveryConfig::default()
            }
            .validate()
            .is_err()
        );
    }
}
