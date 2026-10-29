use std::{io, net::SocketAddr, path::Path, sync::Arc, time::Duration};

use crate::auth::Authenticator;
use crate::inbound::InboundAcceptor;
use crate::metrics::InboundMetrics;
use mail_queue::Queue;
use rustls::{ServerConfig, pki_types::CertificateDer};

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub server_name: String,
    pub authenticator: Option<Arc<dyn Authenticator>>,
    pub require_authentication: bool,
    pub tls: Option<Arc<ServerConfig>>,
    pub implicit_tls: bool,
    pub require_tls_for_auth: bool,
    pub enable_8bitmime: bool,
    pub max_connections: usize,
    pub command_timeout: Duration,
    pub max_line_length: usize,
    pub max_recipients: usize,
    pub max_message_bytes: usize,
    pub max_buffered_input: usize,
    pub queue: Option<Queue>,
    pub inbound: Option<Arc<dyn InboundAcceptor>>,
    pub metrics: Arc<InboundMetrics>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:2525".parse().expect("valid default address"),
            server_name: "smtp-test".into(),
            authenticator: None,
            require_authentication: false,
            tls: None,
            implicit_tls: false,
            require_tls_for_auth: true,
            enable_8bitmime: false,
            max_connections: 64,
            command_timeout: Duration::from_secs(30),
            max_line_length: 1_000,
            max_recipients: 100,
            max_message_bytes: 10 * 1024 * 1024,
            max_buffered_input: 64 * 1024,
            queue: None,
            inbound: None,
            metrics: Arc::new(InboundMetrics::default()),
        }
    }
}

impl Config {
    pub fn from_pem_files(
        mut self,
        certificate_path: impl AsRef<Path>,
        private_key_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let certificate_bytes = std::fs::read(certificate_path)?;
        let private_key_bytes = std::fs::read(private_key_path)?;
        let certificates = rustls_pemfile::certs(&mut certificate_bytes.as_slice())
            .collect::<Result<Vec<CertificateDer<'static>>, _>>()
            .map_err(io::Error::other)?;
        if certificates.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "certificate chain is empty",
            ));
        }
        let private_key = rustls_pemfile::private_key(&mut private_key_bytes.as_slice())
            .map_err(io::Error::other)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "private key is missing"))?;
        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)
            .map_err(io::Error::other)?;
        self.tls = Some(Arc::new(server_config));
        Ok(self)
    }
}
