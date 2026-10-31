use std::{
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::auth::{AuthRateLimiter, Authenticator, PostgresAuthRateLimiter, SenderAuthorizer};
use crate::inbound::InboundAcceptor;
use crate::metrics::InboundMetrics;
use mail_queue::Queue;
use rustls::{ServerConfig, pki_types::CertificateDer};
use sha2::{Digest, Sha256};
use shared_types::MAX_CANONICAL_MESSAGE_BYTES;

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub server_name: String,
    pub authenticator: Option<Arc<dyn Authenticator>>,
    pub sender_authorizer: Option<Arc<dyn SenderAuthorizer>>,
    pub require_authentication: bool,
    pub tls: Option<Arc<ServerConfig>>,
    pub implicit_tls: bool,
    pub require_tls_for_auth: bool,
    pub enable_8bitmime: bool,
    pub max_connections: usize,
    pub command_timeout: Duration,
    pub tls_handshake_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub max_command_length: usize,
    pub max_line_length: usize,
    pub max_recipients: usize,
    pub max_message_bytes: usize,
    pub max_buffered_input: usize,
    pub queue: Option<Queue>,
    pub inbound: Option<Arc<dyn InboundAcceptor>>,
    pub metrics: Arc<InboundMetrics>,
    pub auth_rate_limiter: Arc<AuthRateLimiter>,
    pub shared_auth_rate_limiter: Option<Arc<PostgresAuthRateLimiter>>,
    pub tls_material_paths: Option<(PathBuf, PathBuf)>,
    pub tls_reload_state: Option<Arc<RwLock<TlsReloadState>>>,
}

pub struct TlsReloadState {
    config: Arc<ServerConfig>,
    certificate_hash: [u8; 32],
    private_key_hash: [u8; 32],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:2525".parse().expect("valid default address"),
            server_name: "smtp-test".into(),
            authenticator: None,
            sender_authorizer: None,
            require_authentication: false,
            tls: None,
            implicit_tls: false,
            require_tls_for_auth: true,
            enable_8bitmime: false,
            max_connections: 64,
            command_timeout: Duration::from_secs(30),
            tls_handshake_timeout: Duration::from_secs(10),
            shutdown_timeout: Duration::from_secs(10),
            max_command_length: 512,
            max_line_length: 1_000,
            max_recipients: 100,
            max_message_bytes: MAX_CANONICAL_MESSAGE_BYTES,
            max_buffered_input: 64 * 1024,
            queue: None,
            inbound: None,
            metrics: Arc::new(InboundMetrics::default()),
            auth_rate_limiter: Arc::new(AuthRateLimiter::default()),
            shared_auth_rate_limiter: None,
            tls_material_paths: None,
            tls_reload_state: None,
        }
    }
}

impl Config {
    pub fn validate(&self) -> io::Result<()> {
        if self.server_name.is_empty()
            || self.server_name.len() > 255
            || !self
                .server_name
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b' ')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP server name must be 1-255 visible ASCII bytes without spaces",
            ));
        }
        if self.max_connections == 0
            || self.max_line_length == 0
            || self.max_command_length < 2
            || self.max_line_length < 2
            || self.max_buffered_input == 0
            || self.max_recipients == 0
            || self.max_message_bytes == 0
            || self.max_message_bytes > MAX_CANONICAL_MESSAGE_BYTES
            || self.command_timeout.is_zero()
            || self.tls_handshake_timeout.is_zero()
            || self.shutdown_timeout.is_zero()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP limits and timeouts must be greater than zero",
            ));
        }
        if self.implicit_tls && self.tls.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "implicit TLS requires configured certificate material",
            ));
        }
        if self.authenticator.is_some() && self.tls.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP AUTH requires configured TLS certificate material",
            ));
        }
        if self.authenticator.is_some()
            && !self.bind_addr.ip().is_loopback()
            && self.shared_auth_rate_limiter.is_none()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "public SMTP AUTH requires shared PostgreSQL authentication throttling",
            ));
        }
        if self.authenticator.is_some() && !self.require_authentication {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "configuring SMTP AUTH also requires authentication for every submission",
            ));
        }
        if self.authenticator.is_some() && !self.require_tls_for_auth {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP AUTH PLAIN is supported only over TLS",
            ));
        }
        if self.authenticator.is_some() && self.sender_authorizer.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SMTP AUTH requires tenant-scoped sender-domain authorization",
            ));
        }
        if self.require_authentication && self.authenticator.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "required SMTP authentication needs a configured authenticator",
            ));
        }
        if !self.require_authentication
            && !self.bind_addr.ip().is_loopback()
            && self.inbound.is_none()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unauthenticated SMTP may only bind to loopback without an inbound recipient policy",
            ));
        }
        Ok(())
    }

    pub fn from_pem_files(
        mut self,
        certificate_path: impl AsRef<Path>,
        private_key_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let certificate_path = certificate_path.as_ref().to_path_buf();
        let private_key_path = private_key_path.as_ref().to_path_buf();
        let (config, certificate_hash, private_key_hash) =
            load_tls_material(&certificate_path, &private_key_path)?;
        self.tls = Some(config.clone());
        self.tls_reload_state = Some(Arc::new(RwLock::new(TlsReloadState {
            config,
            certificate_hash,
            private_key_hash,
        })));
        self.tls_material_paths = Some((certificate_path, private_key_path));
        Ok(self)
    }

    pub fn current_tls_config(&self) -> io::Result<Option<Arc<ServerConfig>>> {
        if let Some(state) = &self.tls_reload_state {
            return state
                .read()
                .map(|state| Some(state.config.clone()))
                .map_err(|_| io::Error::other("TLS reload state lock is poisoned"));
        }
        Ok(self.tls.clone())
    }

    pub fn reload_tls_from_files(&self) -> io::Result<bool> {
        let Some((certificate_path, private_key_path)) = &self.tls_material_paths else {
            return Ok(false);
        };
        let Some(state) = &self.tls_reload_state else {
            return Err(io::Error::other(
                "TLS material paths are set without reload state",
            ));
        };
        let certificate_bytes = std::fs::read(certificate_path)?;
        let private_key_bytes = std::fs::read(private_key_path)?;
        let certificate_hash: [u8; 32] = Sha256::digest(&certificate_bytes).into();
        let private_key_hash: [u8; 32] = Sha256::digest(&private_key_bytes).into();
        {
            let current = state
                .read()
                .map_err(|_| io::Error::other("TLS reload state lock is poisoned"))?;
            if current.certificate_hash == certificate_hash
                && current.private_key_hash == private_key_hash
            {
                return Ok(false);
            }
        }
        let config = build_tls_config(&certificate_bytes, &private_key_bytes)?;
        let mut current = state
            .write()
            .map_err(|_| io::Error::other("TLS reload state lock is poisoned"))?;
        current.config = config;
        current.certificate_hash = certificate_hash;
        current.private_key_hash = private_key_hash;
        Ok(true)
    }
}

fn load_tls_material(
    certificate_path: &Path,
    private_key_path: &Path,
) -> io::Result<(Arc<ServerConfig>, [u8; 32], [u8; 32])> {
    let certificate_bytes = std::fs::read(certificate_path)?;
    let private_key_bytes = std::fs::read(private_key_path)?;
    let config = build_tls_config(&certificate_bytes, &private_key_bytes)?;
    Ok((
        config,
        Sha256::digest(&certificate_bytes).into(),
        Sha256::digest(&private_key_bytes).into(),
    ))
}

fn build_tls_config(
    certificate_bytes: &[u8],
    private_key_bytes: &[u8],
) -> io::Result<Arc<ServerConfig>> {
    let certificates = rustls_pemfile::certs(&mut &certificate_bytes[..])
        .collect::<Result<Vec<CertificateDer<'static>>, _>>()
        .map_err(io::Error::other)?;
    if certificates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "certificate chain is empty",
        ));
    }
    let private_key = rustls_pemfile::private_key(&mut &private_key_bytes[..])
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "private key is missing"))?;
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .map_err(io::Error::other)?;
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::{AuthenticatedIdentity, AuthenticationError, Authenticator, SenderAuthorizer};
    use async_trait::async_trait;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    struct CertificateFiles {
        directory: PathBuf,
        certificate: PathBuf,
        private_key: PathBuf,
    }

    impl CertificateFiles {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!("smtp-tls-{}", Uuid::new_v4()));
            fs::create_dir(&directory).expect("create temporary TLS directory");
            Self {
                certificate: directory.join("certificate.pem"),
                private_key: directory.join("private-key.pem"),
                directory,
            }
        }

        fn write_pair(&self) {
            let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
                .expect("generate test certificate");
            fs::write(&self.certificate, certified.cert.pem()).expect("write test certificate");
            fs::write(&self.private_key, certified.key_pair.serialize_pem())
                .expect("write test private key");
        }
    }

    impl Drop for CertificateFiles {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.certificate);
            let _ = fs::remove_file(&self.private_key);
            let _ = fs::remove_dir(&self.directory);
        }
    }

    #[test]
    fn reloads_valid_tls_material_and_retains_previous_config_on_invalid_rotation() {
        let files = CertificateFiles::new();
        files.write_pair();
        let config = Config::default()
            .from_pem_files(&files.certificate, &files.private_key)
            .expect("load initial certificate");
        let initial = config.current_tls_config().unwrap().unwrap();
        assert!(!config.reload_tls_from_files().unwrap());

        files.write_pair();
        assert!(config.reload_tls_from_files().unwrap());
        let rotated = config.current_tls_config().unwrap().unwrap();
        assert!(!std::sync::Arc::ptr_eq(&initial, &rotated));
        assert!(!config.reload_tls_from_files().unwrap());

        fs::write(&files.certificate, "invalid certificate").unwrap();
        assert!(config.reload_tls_from_files().is_err());
        let retained = config.current_tls_config().unwrap().unwrap();
        assert!(std::sync::Arc::ptr_eq(&rotated, &retained));
    }

    struct TestAuthenticator;

    #[async_trait]
    impl Authenticator for TestAuthenticator {
        async fn authenticate(
            &self,
            _username: &str,
            _password: &str,
        ) -> Result<Option<AuthenticatedIdentity>, AuthenticationError> {
            Ok(None)
        }
    }

    #[async_trait]
    impl SenderAuthorizer for TestAuthenticator {
        async fn authorize_sender(
            &self,
            _tenant_id: uuid::Uuid,
            _sender: &str,
        ) -> Result<bool, AuthenticationError> {
            Ok(false)
        }
    }

    #[test]
    fn refuses_public_smtp_auth_without_shared_throttling() {
        let files = CertificateFiles::new();
        files.write_pair();
        let mut config = Config::default()
            .from_pem_files(&files.certificate, &files.private_key)
            .expect("load test certificate");
        config.bind_addr = "0.0.0.0:2525".parse().unwrap();
        config.authenticator = Some(std::sync::Arc::new(TestAuthenticator));
        config.sender_authorizer = Some(std::sync::Arc::new(TestAuthenticator));
        config.require_authentication = true;

        let error = config
            .validate()
            .expect_err("public AUTH must need shared throttling");
        assert!(error.to_string().contains("shared PostgreSQL"));
    }

    #[test]
    fn rejects_server_names_that_could_inject_smtp_replies() {
        for server_name in ["", "smtp\r\n250-AUTH PLAIN", "smtp server"] {
            let config = Config {
                server_name: server_name.into(),
                ..Config::default()
            };
            assert!(config.validate().is_err(), "accepted {server_name:?}");
        }
    }

    #[test]
    fn bounds_the_global_message_size_limit() {
        let config = Config {
            max_message_bytes: super::MAX_CANONICAL_MESSAGE_BYTES + 1,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }
}
