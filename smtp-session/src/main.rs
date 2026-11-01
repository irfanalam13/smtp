use std::time::Duration;
use std::{io, net::SocketAddr, sync::Arc};

use mail_queue::{Queue, QueueConfig};
use smtp_ingress::{
    Config, ListenerProfile, Server,
    email_auth::EmailAuthenticator,
    inbound::PostgresInboundStore,
    scanner::{ClamdScanner, SpamAssassinScanner},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener_profile = match std::env::var("SMTP_LISTENER_PROFILE")
        .unwrap_or_else(|_| "loopback-development".to_owned())
        .as_str()
    {
        "loopback-development" => ListenerProfile::LoopbackDevelopment,
        "inbound-mta" => ListenerProfile::InboundMta,
        "submission" => ListenerProfile::Submission,
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported SMTP_LISTENER_PROFILE: {other}"),
            )
            .into());
        }
    };
    let mut config = Config {
        listener_profile,
        ..Config::default()
    };
    config.implicit_tls = env_bool("SMTP_IMPLICIT_TLS", false)?;
    if let Some(address) = std::env::var_os("SMTP_BIND_ADDR") {
        config.bind_addr = address
            .to_string_lossy()
            .parse::<SocketAddr>()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    } else {
        config.bind_addr = match (listener_profile, config.implicit_tls) {
            (ListenerProfile::InboundMta, _) => "0.0.0.0:25".parse().expect("valid SMTP port"),
            (ListenerProfile::Submission, true) => "0.0.0.0:465".parse().expect("valid SMTP port"),
            (ListenerProfile::Submission, false) => "0.0.0.0:587".parse().expect("valid SMTP port"),
            (ListenerProfile::LoopbackDevelopment, _) => config.bind_addr,
        };
    }
    config.max_message_bytes = env_usize("SMTP_MAX_MESSAGE_BYTES", config.max_message_bytes)?;
    let inbound_enabled = env_bool("SMTP_INBOUND_ENABLED", false)?;

    match (
        std::env::var_os("SMTP_TLS_CERT"),
        std::env::var_os("SMTP_TLS_KEY"),
    ) {
        (Some(certificate), Some(private_key)) => {
            config = config.from_pem_files(certificate, private_key)?;
        }
        (None, None) => {}
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP_TLS_CERT and SMTP_TLS_KEY must be configured together",
            )
            .into());
        }
    }

    config.enable_8bitmime = env_bool("SMTP_ENABLE_8BITMIME", false)?;
    let auth_enabled = env_bool("SMTP_AUTH_ENABLED", false)?;
    config.require_authentication = env_bool("SMTP_REQUIRE_AUTHENTICATION", auth_enabled)?;
    config.require_tls_for_auth = true;

    if std::env::var_os("DATABASE_URL").is_some() {
        config.queue = Some(Queue::connect(QueueConfig::from_env()?).await?);
    }
    if inbound_enabled {
        let database_url = std::env::var("DATABASE_URL").map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "DATABASE_URL is required when SMTP_INBOUND_ENABLED=true",
            )
        })?;
        let inbound_limit = env_usize("SMTP_INBOUND_MAX_MESSAGE_BYTES", config.max_message_bytes)?;
        if inbound_limit == 0 || inbound_limit > config.max_message_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP_INBOUND_MAX_MESSAGE_BYTES must be between 1 and SMTP_MAX_MESSAGE_BYTES",
            )
            .into());
        }
        let scanner_addr = std::env::var("SMTP_CLAMD_ADDR")
            .ok()
            .map(|value| {
                value.parse::<SocketAddr>().map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("SMTP_CLAMD_ADDR must be an IP:port socket address: {error}"),
                    )
                })
            })
            .transpose()?;
        let spam_scanner_addr = std::env::var("SMTP_SPAMD_ADDR")
            .ok()
            .map(|value| {
                value.parse::<SocketAddr>().map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("SMTP_SPAMD_ADDR must be an IP:port socket address: {error}"),
                    )
                })
            })
            .transpose()?;
        if !config.bind_addr.ip().is_loopback() && scanner_addr.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP_CLAMD_ADDR is required for non-loopback inbound SMTP",
            )
            .into());
        }
        if !config.bind_addr.ip().is_loopback() && spam_scanner_addr.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP_SPAMD_ADDR is required for non-loopback inbound SMTP",
            )
            .into());
        }
        let email_authenticator = EmailAuthenticator::from_system_configuration()?;
        let mut inbound = PostgresInboundStore::connect(&database_url, inbound_limit)
            .await?
            .with_email_authenticator(email_authenticator);
        let rate_limit_key = std::env::var("SMTP_INBOUND_RATE_LIMIT_HMAC_KEY").ok();
        if !config.bind_addr.ip().is_loopback() && rate_limit_key.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP_INBOUND_RATE_LIMIT_HMAC_KEY is required for non-loopback inbound SMTP",
            )
            .into());
        }
        if let Some(rate_limit_key) = rate_limit_key {
            let messages_per_hour = env_usize("SMTP_INBOUND_MAX_MESSAGES_PER_HOUR", 10_000)?;
            let messages_per_hour = u32::try_from(messages_per_hour).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "SMTP_INBOUND_MAX_MESSAGES_PER_HOUR exceeds supported range",
                )
            })?;
            inbound = inbound.with_rate_limiter(rate_limit_key.as_bytes(), messages_per_hour)?;
        }
        if let Some(scanner_addr) = scanner_addr {
            let scanner_timeout_seconds = env_usize("SMTP_CLAMD_TIMEOUT_SECONDS", 10)?;
            if !(1..=60).contains(&scanner_timeout_seconds) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "SMTP_CLAMD_TIMEOUT_SECONDS must be between 1 and 60",
                )
                .into());
            }
            inbound = inbound.with_scanner(ClamdScanner::new(
                scanner_addr,
                Duration::from_secs(scanner_timeout_seconds as u64),
            )?);
        }
        if let Some(spam_scanner_addr) = spam_scanner_addr {
            let scanner_timeout_seconds = env_usize("SMTP_SPAMD_TIMEOUT_SECONDS", 10)?;
            if !(1..=60).contains(&scanner_timeout_seconds) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "SMTP_SPAMD_TIMEOUT_SECONDS must be between 1 and 60",
                )
                .into());
            }
            inbound = inbound.with_spam_scanner(SpamAssassinScanner::new(
                spam_scanner_addr,
                Duration::from_secs(scanner_timeout_seconds as u64),
            )?);
        }
        config.inbound = Some(Arc::new(inbound));
    } else if std::env::var_os("SMTP_CLAMD_ADDR").is_some()
        || std::env::var_os("SMTP_SPAMD_ADDR").is_some()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SMTP_CLAMD_ADDR and SMTP_SPAMD_ADDR require SMTP_INBOUND_ENABLED=true",
        )
        .into());
    }
    if auth_enabled {
        let database_url = std::env::var("DATABASE_URL").map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "DATABASE_URL is required when SMTP_AUTH_ENABLED=true",
            )
        })?;
        let authenticator =
            Arc::new(smtp_ingress::PostgresAuthenticator::connect(&database_url).await?);
        let rate_limit_key = std::env::var("SMTP_RATE_LIMIT_HMAC_KEY").map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP_RATE_LIMIT_HMAC_KEY is required when SMTP_AUTH_ENABLED=true",
            )
        })?;
        if rate_limit_key.len() < 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP_RATE_LIMIT_HMAC_KEY must contain at least 32 bytes",
            )
            .into());
        }
        config.shared_auth_rate_limiter = Some(Arc::new(
            authenticator.shared_rate_limiter(rate_limit_key.as_bytes())?,
        ));
        config.authenticator = Some(authenticator.clone());
        config.sender_authorizer = Some(authenticator);
    }
    config.validate()?;

    let tls_reload_task = config.tls_material_paths.as_ref().map(|_| {
        let reload_config = config.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                match reload_config.reload_tls_from_files() {
                    Ok(true) => eprintln!("SMTP TLS certificate material reloaded"),
                    Ok(false) => {}
                    Err(error) => {
                        eprintln!("SMTP TLS certificate reload failed; retaining active certificate: {error}")
                    }
                }
            }
        })
    });
    let server = Server::bind(config).await?;
    eprintln!("SMTP ingress listening on {}", server.local_addr()?);
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        server
            .run(async move {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if let Err(error) = result {
                            eprintln!("failed to receive SMTP shutdown signal: {error}");
                        }
                    }
                    _ = terminate.recv() => {}
                }
            })
            .await?;
    }
    #[cfg(not(unix))]
    {
        server
            .run(async {
                if let Err(error) = tokio::signal::ctrl_c().await {
                    eprintln!("failed to receive SMTP shutdown signal: {error}");
                }
            })
            .await?;
    }
    if let Some(task) = tls_reload_task {
        task.abort();
        let _ = task.await;
    }
    Ok(())
}

fn env_bool(name: &str, default: bool) -> Result<bool, io::Error> {
    let Some(value) = std::env::var_os(name) else {
        return Ok(default);
    };
    match value.to_string_lossy().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be true or false"),
        )),
    }
}

fn env_usize(name: &str, default: usize) -> Result<usize, io::Error> {
    let Some(value) = std::env::var_os(name) else {
        return Ok(default);
    };
    value
        .to_string_lossy()
        .parse::<usize>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, format!("{name}: {error}")))
}
