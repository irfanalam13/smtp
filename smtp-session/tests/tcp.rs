use std::{net::SocketAddr, time::Duration};

use smtp_ingress::{Config, Server};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::oneshot,
    time::timeout,
};

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

async fn line(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> String {
    let mut value = String::new();
    timeout(Duration::from_secs(2), reader.read_line(&mut value))
        .await
        .expect("response timeout")
        .expect("read response");
    value
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
        .write_all(b"Subject: test\r\n..leading-dot\r\n.\r\n")
        .await
        .unwrap();
    assert!(line(&mut reader).await.starts_with("250 "));
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
    writer.write_all(b"12345\r\n").await.unwrap();
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
