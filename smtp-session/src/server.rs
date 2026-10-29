use std::{future::Future, sync::Arc};

use tokio::{
    io::BufReader,
    net::{TcpListener, TcpStream},
    sync::{Semaphore, oneshot},
    task::JoinSet,
};
use tokio_rustls::TlsAcceptor;

use crate::{config::Config, error::SmtpError, session::Session};

pub struct Server {
    listener: TcpListener,
    config: Config,
    connections: Arc<Semaphore>,
}

impl Server {
    pub async fn bind(config: Config) -> Result<Self, SmtpError> {
        if config.implicit_tls && config.tls.is_none() {
            return Err(SmtpError::Tls(
                "implicit TLS requires configured certificate material".into(),
            ));
        }
        let listener = TcpListener::bind(config.bind_addr).await?;
        Ok(Self {
            listener,
            connections: Arc::new(Semaphore::new(config.max_connections)),
            config,
        })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr, SmtpError> {
        Ok(self.listener.local_addr()?)
    }

    pub async fn run(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), SmtpError> {
        let Server {
            listener,
            config,
            connections,
        } = self;
        let mut tasks = JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                accepted = listener.accept() => {
                    let (stream, peer) = accepted?;
                    let permit = match connections.clone().try_acquire_owned() {
                        Ok(permit) => permit,
                        Err(_) => {
                            let _ = reject_busy(stream).await;
                            continue;
                        }
                    };
                    let task_config = config.clone();
                    tasks.spawn(async move {
                        let _permit = permit;
                        let _ = handle_connection(stream, task_config).await;
                        eprintln!("smtp connection closed: {}", sanitize_peer(peer));
                    });
                }
            }
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        Ok(())
    }
}

pub async fn shutdown_channel() -> (oneshot::Sender<()>, oneshot::Receiver<()>) {
    oneshot::channel()
}

async fn handle_connection(stream: TcpStream, config: Config) -> Result<(), SmtpError> {
    if config.implicit_tls {
        let tls = config
            .tls
            .clone()
            .ok_or_else(|| SmtpError::Tls("implicit TLS requires a certificate".into()))?;
        let stream = TlsAcceptor::from(tls)
            .accept(stream)
            .await
            .map_err(|error| SmtpError::Tls(error.to_string()))?;
        return handle_stream(stream, config, true).await;
    }
    handle_stream(stream, config, false).await
}

async fn handle_stream<S>(stream: S, config: Config, tls_active: bool) -> Result<(), SmtpError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::with_capacity(config.max_buffered_input, reader);
    let mut writer = writer;
    config
        .metrics
        .active_connections
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let metrics = config.metrics.clone();
    let result = Session::new_with_transport(config, tls_active)
        .run(&mut reader, &mut writer)
        .await
        .map(|_| ());
    metrics
        .active_connections
        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    result
}

async fn reject_busy(mut stream: TcpStream) -> Result<(), SmtpError> {
    use tokio::io::AsyncWriteExt;
    stream
        .write_all(b"421 4.3.2 too many connections\r\n")
        .await?;
    Ok(())
}

fn sanitize_peer(peer: std::net::SocketAddr) -> String {
    peer.to_string().replace(['\r', '\n'], "")
}
