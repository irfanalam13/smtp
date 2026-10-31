use std::{net::SocketAddr, sync::Arc, time::Duration};

use smtp_ingress::{
    AuthenticatedIdentity, AuthenticationError, Authenticator, Config, SenderAuthorizer, Server,
    Session,
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::oneshot,
    time::timeout,
};
use tokio_rustls::{TlsConnector, rustls};

async fn start(config: Config) -> (SocketAddr, oneshot::Sender<()>) {
    let server = Server::bind(config).await.expect("bind test server");
    let address = server.local_addr().expect("local address");
    let (shutdown, signal) = oneshot::channel();
    tokio::spawn(async move {
        server
            .run(async move {
                let _ = signal.await;
            })
            .await
            .expect("server run");
    });
    (address, shutdown)
}

async fn line(reader: &mut (impl AsyncBufRead + Unpin)) -> String {
    let mut value = String::new();
    timeout(Duration::from_secs(2), reader.read_line(&mut value))
        .await
        .expect("response timeout")
        .expect("read response");
    value
}

async fn response(reader: &mut (impl AsyncBufRead + Unpin)) -> Vec<String> {
    let mut lines = Vec::new();
    loop {
        let value = line(reader).await;
        let done = value.starts_with("250 ");
        lines.push(value);
        if done || !lines.last().unwrap().starts_with("250-") {
            return lines;
        }
    }
}

fn tls_config() -> (Arc<rustls::ServerConfig>, Arc<rustls::ClientConfig>) {
    static INSTALL_PROVIDER: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INSTALL_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
        .expect("generate test certificate");
    let certificate = certified.cert.der().clone();
    let private_key =
        rustls::pki_types::PrivateKeyDer::Pkcs8(certified.key_pair.serialize_der().into());
    let server = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], private_key)
        .expect("build test server TLS config");
    let mut roots = rustls::RootCertStore::empty();
    roots.add(certificate).expect("add test root");
    let client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    (Arc::new(server), Arc::new(client))
}

struct TestAuthenticator;

#[async_trait::async_trait]
impl Authenticator for TestAuthenticator {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<AuthenticatedIdentity>, AuthenticationError> {
        Ok(
            (username == "account" && password == "secret").then(|| AuthenticatedIdentity {
                tenant_id: uuid::Uuid::nil(),
                username: "account".into(),
            }),
        )
    }
}

struct TestSenderAuthorizer;

#[async_trait::async_trait]
impl SenderAuthorizer for TestSenderAuthorizer {
    async fn authorize_sender(
        &self,
        _tenant_id: uuid::Uuid,
        sender: &str,
    ) -> Result<bool, AuthenticationError> {
        Ok(sender.ends_with("@example.test"))
    }
}

#[tokio::test]
async fn completes_transaction_and_dot_unstuffs_data() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    assert!(line(&mut reader).await.starts_with("220 "));
    writer.write_all(b"EHLO client\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("250-"));
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"MAIL FROM:<sender@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer
        .write_all(b"Subject: test\r\n\r\n..leading-dot\r\n.\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"QUIT\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("221 "));

    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn enforces_esmtp_negotiation_declared_and_actual_size_limits_and_7bit_content() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        max_message_bytes: 16,
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    assert!(line(&mut reader).await.starts_with("220 "));
    writer.write_all(b"HELO client\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"MAIL FROM:<sender@example.test> SIZE=1\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("555 "));
    writer.write_all(b"EHLO client\r\n").await.unwrap();
    let ehlo = response(&mut reader).await.join("");
    assert!(ehlo.contains("SIZE 16"));
    assert!(!ehlo.contains("8BITMIME"));

    writer
        .write_all(b"MAIL FROM:<sender@example.test> BODY=7BIT\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("555 "));
    writer
        .write_all(b"MAIL FROM:<sender@example.test> SIZE=17\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("552 "));
    writer
        .write_all(b"MAIL FROM:<sender@example.test> SIZE=16\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer
        .write_all(b"12345678901234567\r\n.\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("552 "));

    writer
        .write_all(b"MAIL FROM:<sender@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer.write_all(b"high-bit: \x80\r\n.\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("554 "));
    writer.write_all(b"QUIT\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("221 "));
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn eight_bitmime_preserves_octets_and_dot_unstuffs_all_leading_dots() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        enable_8bitmime: true,
        ..Config::default()
    };
    let (client, server) = tokio::io::duplex(4096);
    let (server_reader, mut server_writer) = tokio::io::split(server);
    let server_task = tokio::spawn(async move {
        let mut reader = BufReader::new(server_reader);
        Session::new(config)
            .run(&mut reader, &mut server_writer)
            .await
            .unwrap()
    });

    let (reader, mut writer) = tokio::io::split(client);
    let mut reader = BufReader::new(reader);
    assert!(line(&mut reader).await.starts_with("220 "));
    writer.write_all(b"EHLO client\r\n").await.unwrap();
    assert!(response(&mut reader).await.join("").contains("8BITMIME"));
    writer
        .write_all(b"MAIL FROM:<sender@example.test> BODY=8BITMIME\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer
        .write_all(b"Subject: caf\xe9\r\n\r\nbody\r\n.\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("554 "));

    writer
        .write_all(b"MAIL FROM:<sender@example.test> BODY=8BITMIME\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer
        .write_all(b"Subject: test\r\n\r\ncaf\xe9\r\n.leading-dot\r\n..double-dot\r\n.\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"QUIT\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("221 "));
    let result = server_task.await.unwrap();
    assert_eq!(result.messages.len(), 1);
    assert_eq!(
        result.messages[0].body,
        b"Subject: test\r\n\r\ncaf\xe9\r\nleading-dot\r\n.double-dot\r\n"
    );
}

#[tokio::test]
async fn rejects_bare_crlf_violations_without_losing_command_synchronization() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    assert!(line(&mut reader).await.starts_with("220 "));
    writer.write_all(b"NOOP\rX\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("500 "));
    writer.write_all(b"NOOP\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("500 "));
    writer.write_all(b"NOOP\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"QUIT\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("221 "));
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn enforces_rfc_command_and_data_line_lengths_including_crlf() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        max_message_bytes: 2_000,
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    assert!(line(&mut reader).await.starts_with("220 "));

    writer
        .write_all(format!("NOOP {}\r\n", "a".repeat(505)).as_bytes())
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(format!("NOOP {}\r\n", "a".repeat(506)).as_bytes())
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("500 "));

    writer.write_all(b"EHLO client\r\n").await.unwrap();
    let _ = response(&mut reader).await;
    writer
        .write_all(b"MAIL FROM:<sender@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer
        .write_all(format!("\r\n{}\r\n.\r\n", "a".repeat(998)).as_bytes())
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));

    writer
        .write_all(b"MAIL FROM:<sender@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer
        .write_all(format!("\r\n{}\r\n.\r\n", "a".repeat(999)).as_bytes())
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("552 "));
    writer.write_all(b"QUIT\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("221 "));
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn rejects_bad_sequence_limits_and_unsupported_commands() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        max_recipients: 1,
        max_line_length: 32,
        max_message_bytes: 4,
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let _ = line(&mut reader).await;
    writer
        .write_all(b"RCPT TO:<a@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("503 "));
    writer
        .write_all(b"EHLO client\r\nMAIL FROM:<a@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250-"));
    assert!(line(&mut reader).await.starts_with("250 "));
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<a@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
    writer
        .write_all(b"RCPT TO:<b@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("452 "));
    writer.write_all(b"AUTH PLAIN\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("502 "));
    writer.write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("354 "));
    writer.write_all(b"12345\r\n.\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("552 "));

    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn supports_concurrent_loopback_clients_and_shutdown() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        max_connections: 8,
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let mut clients = Vec::new();
    for _ in 0..4 {
        clients.push(tokio::spawn(async move {
            let stream = TcpStream::connect(address).await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            assert!(line(&mut reader).await.starts_with("220 "));
            writer.write_all(b"QUIT\r\n").await.unwrap();
            assert!(line(&mut reader).await.starts_with("221 "));
        }));
    }
    for client in clients {
        client.await.unwrap();
    }
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn starttls_resets_session_and_allows_auth_only_after_upgrade() {
    let (server_tls, client_tls) = tls_config();
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        tls: Some(server_tls),
        authenticator: Some(Arc::new(TestAuthenticator)),
        sender_authorizer: Some(Arc::new(TestSenderAuthorizer)),
        require_authentication: true,
        ..Config::default()
    };
    let metrics = config.metrics.clone();
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    assert!(line(&mut reader).await.starts_with("220 "));
    writer.write_all(b"EHLO client\r\n").await.unwrap();
    let ehlo = response(&mut reader).await.join("");
    assert!(ehlo.contains("STARTTLS"));
    assert!(!ehlo.contains("AUTH PLAIN"));

    writer.write_all(b"AUTH PLAIN\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("538 "));
    writer.write_all(b"STARTTLS\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("220 "));

    let plain = reader.into_inner().reunite(writer).unwrap();
    let connector = TlsConnector::from(client_tls);
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let mut tls_reader = BufReader::new(
        connector
            .connect(server_name, plain)
            .await
            .expect("complete STARTTLS handshake"),
    );
    tls_reader
        .get_mut()
        .write_all(b"MAIL FROM:<sender@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut tls_reader).await.starts_with("503 "));
    tls_reader
        .get_mut()
        .write_all(b"EHLO client\r\n")
        .await
        .unwrap();
    let post_tls_ehlo = response(&mut tls_reader).await.join("");
    assert!(post_tls_ehlo.contains("AUTH PLAIN"));
    assert!(!post_tls_ehlo.contains("STARTTLS"));

    tls_reader
        .get_mut()
        .write_all(b"AUTH PLAIN AGFjY291bnQAd3Jvbmc=\r\n")
        .await
        .unwrap();
    assert!(line(&mut tls_reader).await.starts_with("535 "));
    tls_reader
        .get_mut()
        .write_all(b"AUTH PLAIN AGFjY291bnQAc2VjcmV0\r\n")
        .await
        .unwrap();
    assert!(line(&mut tls_reader).await.starts_with("235 "));
    tls_reader
        .get_mut()
        .write_all(b"MAIL FROM:<spoof@unverified.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut tls_reader).await.starts_with("550 "));
    tls_reader
        .get_mut()
        .write_all(b"MAIL FROM:<sender@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut tls_reader).await.starts_with("250 "));
    tls_reader
        .get_mut()
        .write_all(b"RCPT TO:<recipient@example.test>\r\n")
        .await
        .unwrap();
    assert!(line(&mut tls_reader).await.starts_with("250 "));
    tls_reader.get_mut().write_all(b"DATA\r\n").await.unwrap();
    assert!(line(&mut tls_reader).await.starts_with("354 "));
    tls_reader
        .get_mut()
        .write_all(b"Subject: secure submission\r\n\r\nmessage\r\n.\r\n")
        .await
        .unwrap();
    assert!(line(&mut tls_reader).await.starts_with("250 "));
    tls_reader.get_mut().write_all(b"QUIT\r\n").await.unwrap();
    assert!(line(&mut tls_reader).await.starts_with("221 "));

    shutdown.send(()).unwrap();
    let metrics = metrics.snapshot();
    assert_eq!(metrics.starttls_upgrades, 1);
    assert_eq!(metrics.authentication_attempts, 3);
    assert_eq!(metrics.authentication_failures, 2);
}

#[tokio::test]
async fn failed_starttls_handshake_closes_without_plaintext_fallback() {
    let (server_tls, _) = tls_config();
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        tls: Some(server_tls),
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let _ = line(&mut reader).await;
    writer.write_all(b"EHLO client\r\n").await.unwrap();
    let _ = response(&mut reader).await;
    writer.write_all(b"STARTTLS\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("220 "));
    writer.write_all(b"NOT_TLS").await.unwrap();
    let mut response = Vec::new();
    timeout(Duration::from_secs(2), reader.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(!response.windows(4).any(|window| window == b"220 "));
    assert!(!response.windows(4).any(|window| window == b"454 "));
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn rejects_pipelined_plaintext_after_starttls() {
    let (server_tls, _) = tls_config();
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        tls: Some(server_tls),
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let _ = line(&mut reader).await;
    writer.write_all(b"EHLO client\r\n").await.unwrap();
    let _ = response(&mut reader).await;
    writer.write_all(b"STARTTLS\r\nNOOP\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("554 "));
    assert_eq!(
        timeout(Duration::from_secs(2), line(&mut reader))
            .await
            .unwrap(),
        ""
    );
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn rejects_public_unauthenticated_submission_and_limits_connections() {
    let unauthenticated = Config {
        bind_addr: "0.0.0.0:0".parse().unwrap(),
        ..Config::default()
    };
    assert!(Server::bind(unauthenticated).await.is_err());

    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        max_connections: 1,
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let first = TcpStream::connect(address).await.unwrap();
    let (reader, _first_writer) = first.into_split();
    let mut first_reader = BufReader::new(reader);
    assert!(line(&mut first_reader).await.starts_with("220 "));
    let second = TcpStream::connect(address).await.unwrap();
    let (second_reader, _second_writer) = second.into_split();
    let mut second_reader = BufReader::new(second_reader);
    assert!(line(&mut second_reader).await.starts_with("421 "));
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn closes_a_slow_client_after_the_command_deadline() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        command_timeout: Duration::from_millis(50),
        ..Config::default()
    };
    let (address, shutdown) = start(config).await;
    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, _writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    assert!(line(&mut reader).await.starts_with("220 "));
    assert_eq!(
        timeout(Duration::from_secs(2), line(&mut reader))
            .await
            .unwrap(),
        ""
    );
    shutdown.send(()).unwrap();
}

#[tokio::test]
async fn graceful_shutdown_drains_a_session_until_its_deadline() {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        shutdown_timeout: Duration::from_secs(1),
        ..Config::default()
    };
    let server = Server::bind(config).await.unwrap();
    let address = server.local_addr().unwrap();
    let (shutdown, signal) = oneshot::channel();
    let server_task = tokio::spawn(async move {
        server
            .run(async move {
                let _ = signal.await;
            })
            .await
            .unwrap();
    });

    let stream = TcpStream::connect(address).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    assert!(line(&mut reader).await.starts_with("220 "));
    shutdown.send(()).unwrap();
    writer.write_all(b"QUIT\r\n").await.unwrap();
    assert!(line(&mut reader).await.starts_with("221 "));
    timeout(Duration::from_secs(2), server_task)
        .await
        .expect("server drains the finished session")
        .unwrap();
}
