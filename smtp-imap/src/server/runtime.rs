use super::*;

pub async fn run_from_env() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let certificate = env::var("IMAP_TLS_CERT").map_err(|_| "IMAP_TLS_CERT is required")?;
    let private_key = env::var("IMAP_TLS_KEY").map_err(|_| "IMAP_TLS_KEY is required")?;
    let address: SocketAddr = env::var("IMAP_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:993".to_owned())
        .parse()?;
    let max_connections = env::var("IMAP_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "256".to_owned())
        .parse::<usize>()?;
    if !(1..=MAX_CONNECTIONS).contains(&max_connections) {
        return Err(format!("IMAP_MAX_CONNECTIONS must be between 1 and {MAX_CONNECTIONS}").into());
    }

    let pool = PgPoolOptions::new()
        .max_connections((max_connections.min(32) as u32).max(4))
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await?;
    let rate_limit_key = env::var("SMTP_RATE_LIMIT_HMAC_KEY")
        .map_err(|_| "SMTP_RATE_LIMIT_HMAC_KEY is required for IMAP authentication")?;
    if rate_limit_key.len() < 32 {
        return Err("SMTP_RATE_LIMIT_HMAC_KEY must contain at least 32 bytes".into());
    }
    let authenticator = PostgresAuthenticator::from_pool(pool.clone());
    let auth_rate_limiter = authenticator.shared_rate_limiter(rate_limit_key.as_bytes())?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(load_tls_config(
        Path::new(&certificate),
        Path::new(&private_key),
    )?));
    let listener = TcpListener::bind(address).await?;
    let state = AppState {
        mailbox_store: MailboxStore::from_pool(pool.clone(), MAX_LITERAL_BYTES),
        authenticator,
        auth_rate_limiter,
        pool,
        tls_acceptor,
        max_connections,
    };
    tracing::info!(%address, max_connections, "implicit-TLS IMAP listener started");
    serve(listener, state).await?;
    Ok(())
}

pub(super) async fn serve(listener: TcpListener, state: AppState) -> io::Result<()> {
    let permits = Arc::new(Semaphore::new(state.max_connections));
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            signal = tokio::signal::ctrl_c() => {
                signal?;
                tracing::info!("IMAP shutdown signal received");
                break;
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                let Ok(permit) = permits.clone().try_acquire_owned() else {
                    tracing::warn!(%peer, "IMAP connection limit reached");
                    drop(stream);
                    continue;
                };
                let state = state.clone();
                connections.spawn(async move {
                    if let Err(error) = handle_tls_connection(stream, peer, state, permit).await {
                        tracing::warn!(%peer, %error, "IMAP connection ended with an error");
                    }
                });
            }
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Err(error) = result {
                    tracing::error!(%error, "IMAP connection task failed");
                }
            }
        }
    }
    let drain = async {
        while let Some(result) = connections.join_next().await {
            if let Err(error) = result {
                tracing::error!(%error, "IMAP connection task failed during shutdown");
            }
        }
    };
    if timeout(Duration::from_secs(10), drain).await.is_err() {
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }
    Ok(())
}

pub(super) async fn handle_tls_connection(
    stream: TcpStream,
    peer: SocketAddr,
    state: AppState,
    _permit: OwnedSemaphorePermit,
) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let tls = timeout(TLS_HANDSHAKE_TIMEOUT, state.tls_acceptor.accept(stream))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "TLS handshake timed out"))??;
    handle_connection(BufStream::new(tls), peer, state).await
}

pub(super) async fn handle_connection<S>(
    mut stream: BufStream<S>,
    peer: SocketAddr,
    state: AppState,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    stream
        .write_all(
            format!(
                "* OK {} IMAP4rev1 service ready\r\n",
                env::var("IMAP_SERVER_NAME").unwrap_or_else(|_| "smtp-platform".into())
            )
            .as_bytes(),
        )
        .await?;
    stream.flush().await?;
    let mut connection = Connection::default();
    let mut pending_line = Vec::new();
    loop {
        let line = timeout(
            COMMAND_TIMEOUT,
            read_bounded_line(&mut stream, &mut pending_line, MAX_COMMAND_BYTES),
        )
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "IMAP command read timed out"))?;
        let line = match line {
            Ok(Some(line)) => line,
            Ok(None) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                stream
                    .write_all(b"* BAD command line exceeds limit\r\n")
                    .await?;
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        if !line.ends_with(b"\r\n") {
            stream.write_all(b"* BAD CRLF required\r\n").await?;
            return Ok(());
        }
        let command = match parse_command_line(&line) {
            Ok(command) => command,
            Err(error) => {
                let tag = line
                    .split(|byte| byte.is_ascii_whitespace())
                    .next()
                    .filter(|tag| !tag.is_empty() && tag.len() <= 64)
                    .and_then(|tag| std::str::from_utf8(tag).ok())
                    .filter(|tag| tag.bytes().all(|byte| (0x21..=0x7e).contains(&byte)));
                if let Some(tag) = tag {
                    write_tagged(&mut stream, tag, "BAD", parse_error_message(&error)).await?;
                } else {
                    stream.write_all(b"* BAD malformed command\r\n").await?;
                }
                continue;
            }
        };
        let close = execute_command(
            &mut stream,
            &mut connection,
            &state,
            peer,
            &command,
            std::str::from_utf8(&line)
                .expect("command parser verified UTF-8")
                .trim_end_matches("\r\n"),
        )
        .await?;
        if close {
            stream.shutdown().await?;
            return Ok(());
        }
    }
}

pub(super) async fn read_bounded_line<S: AsyncBufRead + Unpin>(
    stream: &mut S,
    line: &mut Vec<u8>,
    max_bytes: usize,
) -> io::Result<Option<Vec<u8>>> {
    loop {
        let available = stream.fill_buf().await?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "connection closed mid-command",
                ))
            };
        }
        let count = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        if line.len().saturating_add(count) > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IMAP line exceeds configured bound",
            ));
        }
        let has_newline = available[count - 1] == b'\n';
        line.extend_from_slice(&available[..count]);
        stream.consume(count);
        if has_newline {
            return Ok(Some(std::mem::take(line)));
        }
    }
}

pub(super) fn load_tls_config(
    certificate_path: &Path,
    private_key_path: &Path,
) -> io::Result<ServerConfig> {
    let certificate_bytes = std::fs::read(certificate_path)?;
    let private_key_bytes = std::fs::read(private_key_path)?;
    let certificates = CertificateDer::pem_slice_iter(&certificate_bytes)
        .collect::<Result<Vec<CertificateDer<'static>>, _>>()
        .map_err(io::Error::other)?;
    if certificates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "TLS certificate chain is empty",
        ));
    }
    let key = rustls::pki_types::PrivateKeyDer::from_pem_slice(&private_key_bytes)
        .map_err(io::Error::other)?;
    ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map_err(io::Error::other)
}

pub(super) async fn write_tagged<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    tag: &str,
    status: &str,
    text: &str,
) -> io::Result<()> {
    stream
        .write_all(format!("{tag} {status} {text}\r\n").as_bytes())
        .await
}

pub(super) fn parse_error_message(error: &ParseError) -> &'static str {
    match error {
        ParseError::LineTooLong | ParseError::LiteralTooLarge => "command or literal too large",
        ParseError::InvalidEncoding | ParseError::InvalidQuotedString => "invalid command encoding",
        _ => "invalid command syntax",
    }
}

pub(super) async fn execute_command<S>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    peer: SocketAddr,
    command: &CommandLine,
    original_line: &str,
) -> io::Result<bool>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    match command.name.as_str() {
        "CAPABILITY" => {
            stream
                .write_all(format!("* CAPABILITY {CAPABILITY}\r\n").as_bytes())
                .await?;
            write_tagged(stream, &command.tag, "OK", "CAPABILITY completed").await?;
        }
        "NOOP" => {
            notify_selected(stream, connection, state).await?;
            write_tagged(stream, &command.tag, "OK", "NOOP completed").await?;
        }
        "LOGOUT" => {
            stream.write_all(b"* BYE logging out\r\n").await?;
            write_tagged(stream, &command.tag, "OK", "LOGOUT completed").await?;
            return Ok(true);
        }
        "LOGIN" => login(stream, connection, state, peer, command).await?,
        "AUTHENTICATE" => authenticate_plain(stream, connection, state, peer, command).await?,
        "NAMESPACE" => namespace(stream, connection, command).await?,
        "LIST" | "LSUB" => list_mailboxes(stream, connection, state, command).await?,
        "STATUS" => status(stream, connection, state, command).await?,
        "SELECT" | "EXAMINE" => {
            select_mailbox(
                stream,
                connection,
                state,
                command,
                command.name == "EXAMINE",
            )
            .await?
        }
        "CLOSE" => close_mailbox(stream, connection, state, command).await?,
        "UNSELECT" => unselect_mailbox(stream, connection, command).await?,
        "FETCH" | "UID" => {
            fetch_or_uid(stream, connection, state, command).await?;
        }
        "STORE" => store_flags(stream, connection, state, command).await?,
        "SEARCH" => search(stream, connection, state, command).await?,
        "COPY" | "MOVE" => {
            copy_or_move(stream, connection, state, command, command.name == "MOVE").await?
        }
        "APPEND" => append_message(stream, connection, state, command, original_line).await?,
        "CREATE" | "DELETE" | "RENAME" => mutate_folder(stream, connection, state, command).await?,
        "EXPUNGE" => expunge(stream, connection, state, command).await?,
        "IDLE" => idle(stream, connection, state, command).await?,
        _ => write_tagged(stream, &command.tag, "BAD", "unsupported command").await?,
    }
    stream.flush().await?;
    Ok(false)
}
