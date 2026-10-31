use super::runtime::handle_tls_connection;
use super::*;
use rustls::{ClientConfig, RootCertStore, pki_types::PrivateKeyDer};
use tokio::io::BufReader;
use tokio_rustls::TlsConnector;

#[tokio::test]
async fn bounded_reader_preserves_pipelined_commands() {
    let input = b"A1 NOOP\r\nA2 LOGOUT\r\n";
    let mut reader = BufReader::new(&input[..]);
    let mut pending_line = Vec::new();
    assert_eq!(
        read_bounded_line(&mut reader, &mut pending_line, 16)
            .await
            .unwrap(),
        Some(b"A1 NOOP\r\n".to_vec())
    );
    assert_eq!(
        read_bounded_line(&mut reader, &mut pending_line, 16)
            .await
            .unwrap(),
        Some(b"A2 LOGOUT\r\n".to_vec())
    );
    assert_eq!(
        read_bounded_line(&mut reader, &mut pending_line, 16)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn bounded_reader_rejects_oversized_lines_before_accumulating_them() {
    let input = b"A1 NOOP with too many bytes\r\n";
    let mut reader = BufReader::new(&input[..]);
    let mut pending_line = Vec::new();
    let error = read_bounded_line(&mut reader, &mut pending_line, 8)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn append_options_accept_flags_and_internal_date() {
    let tokens = [
        Token::OpenParen,
        Token::Atom("\\Seen".into()),
        Token::CloseParen,
        Token::Quoted("01-Jan-2024 12:30:45 +0000".into()),
    ];
    let (flags, date) = parse_append_options(&tokens).unwrap();
    assert_eq!(flags, vec![MailboxFlag::Seen]);
    assert_eq!(date.unwrap().to_rfc3339(), "2024-01-01T12:30:45+00:00");
}

#[test]
fn append_options_reject_malformed_flags_or_dates() {
    assert!(parse_append_options(&[Token::OpenParen, Token::Atom("\\Seen".into()),]).is_err());
    assert!(parse_append_options(&[Token::Quoted("not a date".into())]).is_err());
}

#[tokio::test]
async fn tls_connection_serves_capability_noop_and_logout() {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
        .expect("generate test certificate");
    let certificate_der = certificate.cert.der().clone();
    let private_key = PrivateKeyDer::Pkcs8(certificate.key_pair.serialize_der().into());
    let server_config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("select TLS protocol versions")
            .with_no_client_auth()
            .with_single_cert(vec![certificate_der.clone()], private_key)
            .expect("build test server TLS config");
    let mut roots = RootCertStore::empty();
    roots
        .add(certificate_der)
        .expect("add test root certificate");
    let client_config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("select TLS client protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth();

    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://smtp:test@127.0.0.1:1/smtp_imap_test")
        .expect("create lazy test pool");
    let authenticator = PostgresAuthenticator::from_pool(pool.clone());
    let state = AppState {
        mailbox_store: MailboxStore::from_pool(pool.clone(), MAX_LITERAL_BYTES),
        auth_rate_limiter: authenticator
            .shared_rate_limiter(b"01234567890123456789012345678901")
            .expect("create test rate limiter"),
        authenticator,
        pool,
        tls_acceptor: TlsAcceptor::from(Arc::new(server_config)),
        max_connections: 1,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server_task = tokio::spawn(async move {
        let (stream, peer) = listener.accept().await?;
        let permit = Arc::new(Semaphore::new(1))
            .acquire_owned()
            .await
            .map_err(io::Error::other)?;
        handle_tls_connection(stream, peer, state, permit).await
    });

    let stream = TcpStream::connect(address).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost".to_owned()).unwrap();
    let tls = TlsConnector::from(Arc::new(client_config))
        .connect(server_name, stream)
        .await
        .unwrap();
    let mut client = BufReader::new(tls);
    let mut line = Vec::new();
    client.read_until(b'\n', &mut line).await.unwrap();
    assert!(line.starts_with(b"* OK "));

    client
        .get_mut()
        .write_all(b"A1 CAPABILITY\r\n")
        .await
        .unwrap();
    line.clear();
    client.read_until(b'\n', &mut line).await.unwrap();
    assert_eq!(line, format!("* CAPABILITY {CAPABILITY}\r\n").as_bytes());
    line.clear();
    client.read_until(b'\n', &mut line).await.unwrap();
    assert!(line.starts_with(b"A1 OK "));

    client.get_mut().write_all(b"A2 NOOP\r\n").await.unwrap();
    line.clear();
    client.read_until(b'\n', &mut line).await.unwrap();
    assert!(line.starts_with(b"A2 OK "));

    client.get_mut().write_all(b"A3 LOGOUT\r\n").await.unwrap();
    line.clear();
    client.read_until(b'\n', &mut line).await.unwrap();
    assert_eq!(line, b"* BYE logging out\r\n");
    line.clear();
    client.read_until(b'\n', &mut line).await.unwrap();
    assert!(line.starts_with(b"A3 OK "));
    server_task.await.unwrap().unwrap();
}
