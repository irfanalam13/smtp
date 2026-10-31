use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use tokio::{
    io::{AsyncRead, AsyncWrite, BufReader, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{Semaphore, oneshot},
    task::JoinSet,
    time::timeout,
};
use tokio_rustls::{TlsAcceptor, server::TlsStream as ServerTlsStream};

use crate::{config::Config, error::SmtpError, session::Session, session::SessionDisposition};

enum ConnectionStream {
    Plain(TcpStream),
    Tls(Box<ServerTlsStream<TcpStream>>),
}

impl AsyncRead for ConnectionStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_read(context, buffer),
            Self::Tls(stream) => Pin::new(stream).poll_read(context, buffer),
        }
    }
}

impl AsyncWrite for ConnectionStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.as_mut().get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write(context, buffer),
            Self::Tls(stream) => Pin::new(stream).poll_write(context, buffer),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_flush(context),
            Self::Tls(stream) => Pin::new(stream).poll_flush(context),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(context),
            Self::Tls(stream) => Pin::new(stream).poll_shutdown(context),
        }
    }
}

pub struct Server {
    listener: TcpListener,
    config: Config,
    connections: Arc<Semaphore>,
}

impl Server {
    pub async fn bind(config: Config) -> Result<Self, SmtpError> {
        config.validate()?;
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
                        if let Err(error) = handle_connection(stream, task_config, peer).await {
                            eprintln!(
                                "smtp connection failed from {}: {}",
                                sanitize_peer(peer),
                                error
                            );
                        }
                    });
                }
                joined = tasks.join_next(), if !tasks.is_empty() => {
                    if let Some(Err(error)) = joined {
                        eprintln!("smtp connection task failed: {error}");
                    }
                }
            }
        }
        drain_connections(&mut tasks, config.shutdown_timeout).await;
        Ok(())
    }
}

pub async fn shutdown_channel() -> (oneshot::Sender<()>, oneshot::Receiver<()>) {
    oneshot::channel()
}

async fn handle_connection(
    stream: TcpStream,
    config: Config,
    peer: std::net::SocketAddr,
) -> Result<(), SmtpError> {
    let metrics = config.metrics.clone();
    metrics
        .active_connections
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let result = if config.implicit_tls {
        let tls = config
            .current_tls_config()?
            .ok_or_else(|| SmtpError::Tls("implicit TLS requires a certificate".into()))?;
        match timeout(
            config.tls_handshake_timeout,
            TlsAcceptor::from(tls).accept(stream),
        )
        .await
        {
            Ok(Ok(stream)) => {
                run_session(
                    ConnectionStream::Tls(Box::new(stream)),
                    config.clone(),
                    true,
                    peer.ip(),
                    true,
                )
                .await
            }
            Ok(Err(error)) => {
                record_tls_failure(&config);
                Err(SmtpError::Tls(error.to_string()))
            }
            Err(_) => {
                record_tls_failure(&config);
                Err(SmtpError::Timeout)
            }
        }
    } else {
        run_session(
            ConnectionStream::Plain(stream),
            config.clone(),
            false,
            peer.ip(),
            true,
        )
        .await
    };

    metrics
        .active_connections
        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    result
}

async fn run_session(
    mut stream: ConnectionStream,
    config: Config,
    mut tls_active: bool,
    peer_ip: std::net::IpAddr,
    mut send_greeting: bool,
) -> Result<(), SmtpError> {
    loop {
        let (reader_half, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::with_capacity(config.max_buffered_input, reader_half);
        let mut session = Session::new_for_server(config.clone(), tls_active, peer_ip);
        let result = if send_greeting {
            session.run(&mut reader, &mut writer).await?
        } else {
            session.resume_after_tls(&mut reader, &mut writer).await?
        };
        let buffered_plaintext = !reader.buffer().is_empty();
        stream = reader.into_inner().unsplit(writer);

        match result.disposition {
            SessionDisposition::Closed => return Ok(()),
            SessionDisposition::StartTls => {
                if buffered_plaintext {
                    return Err(SmtpError::Tls(
                        "plaintext data was buffered after the STARTTLS command".into(),
                    ));
                }
                let ConnectionStream::Plain(plain_stream) = stream else {
                    return Err(SmtpError::Tls(
                        "STARTTLS cannot be negotiated on an encrypted connection".into(),
                    ));
                };
                let tls = config
                    .current_tls_config()?
                    .ok_or_else(|| SmtpError::Tls("STARTTLS certificate is unavailable".into()))?;
                let tls_stream = match timeout(
                    config.tls_handshake_timeout,
                    TlsAcceptor::from(tls).accept(plain_stream),
                )
                .await
                {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(error)) => {
                        record_tls_failure(&config);
                        return Err(SmtpError::Tls(error.to_string()));
                    }
                    Err(_) => {
                        record_tls_failure(&config);
                        return Err(SmtpError::Timeout);
                    }
                };
                config
                    .metrics
                    .starttls_upgrades
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                stream = ConnectionStream::Tls(Box::new(tls_stream));
                tls_active = true;
                send_greeting = false;
            }
        }
    }
}

fn record_tls_failure(config: &Config) {
    config
        .metrics
        .tls_handshake_failures
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

async fn drain_connections(tasks: &mut JoinSet<()>, drain_timeout: Duration) {
    let drained = timeout(drain_timeout, async {
        while let Some(result) = tasks.join_next().await {
            if let Err(error) = result {
                eprintln!("smtp connection task failed during shutdown: {error}");
            }
        }
    })
    .await;
    if drained.is_err() {
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }
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
