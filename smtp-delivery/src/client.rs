use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use mail_queue::model::ClaimedJob;
use rustls::{ClientConfig, RootCertStore};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf},
    net::TcpStream,
};
use tokio_rustls::{TlsConnector, client::TlsStream};

use crate::{
    config::{DeliveryConfig, TlsPolicy},
    dns::{DestinationPolicy, Resolver, recipient_domain, socket_candidates},
    framing::write_message,
    outcome::{DeliveryError, DeliveryOutcome, classify_status},
    reply::{SmtpReply, read_reply},
};

enum SmtpTransport {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for SmtpTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for SmtpTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

pub struct DeliveryReport {
    pub outcome: DeliveryOutcome,
    pub accepted_recipients: usize,
    pub rejected_recipients: usize,
}

pub async fn deliver<R: Resolver>(
    job: &ClaimedJob,
    config: &DeliveryConfig,
    resolver: &R,
) -> Result<DeliveryReport, DeliveryError> {
    tokio::time::timeout(config.overall_timeout, deliver_inner(job, config, resolver))
        .await
        .map_err(|_| DeliveryError::ConnectTimeout)?
}

async fn deliver_inner<R: Resolver>(
    job: &ClaimedJob,
    config: &DeliveryConfig,
    resolver: &R,
) -> Result<DeliveryReport, DeliveryError> {
    config
        .validate()
        .map_err(|_| DeliveryError::InternalFailure)?;
    if !config.enabled {
        return Err(DeliveryError::Disabled);
    }
    if job.body.len() > config.max_message_bytes {
        return Err(DeliveryError::MessageTooLarge);
    }
    let domain = recipient_domain(job.recipients.first().ok_or(DeliveryError::NoCandidate)?)?;
    let policy = DestinationPolicy {
        require_public: config.require_public_destinations,
        allow_loopback_test: config.allow_loopback_test_destinations,
    };
    let candidates = if let Some(endpoint) = config.test_endpoint {
        if !policy.allows(endpoint.ip()) {
            return Err(DeliveryError::UnsafeDestination);
        }
        vec![(endpoint, "localhost".to_string())]
    } else {
        let mx = tokio::time::timeout(
            config.dns_timeout,
            resolver.resolve_mx(domain, config.dns_timeout, config.max_mx_candidates),
        )
        .await
        .map_err(|_| DeliveryError::Dns)??;
        mx.into_iter()
            .flat_map(|candidate| {
                socket_candidates(&candidate, &policy)
                    .into_iter()
                    .map(move |address| (address, candidate.host.clone()))
            })
            .take(config.max_fallback_attempts)
            .collect()
    };
    if candidates.is_empty() {
        return Err(DeliveryError::UnsafeDestination);
    }

    let mut last_error = DeliveryError::NoCandidate;
    for (address, host) in candidates {
        if config.test_endpoint.is_none() && !policy.allows(address.ip()) {
            last_error = DeliveryError::UnsafeDestination;
            continue;
        }
        match deliver_to(job, config, address, &host).await {
            Ok(report) => return Ok(report),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

async fn deliver_to(
    job: &ClaimedJob,
    config: &DeliveryConfig,
    address: std::net::SocketAddr,
    host: &str,
) -> Result<DeliveryReport, DeliveryError> {
    let stream = tokio::time::timeout(config.connect_timeout, TcpStream::connect(address))
        .await
        .map_err(|_| DeliveryError::ConnectTimeout)?
        .map_err(|_| DeliveryError::ConnectTimeout)?;
    let mut io = SmtpTransport::Plain(stream);
    let mut reader = BufReader::new(io);
    expect_code(&mut reader, config, 220).await?;
    send_command(
        &mut reader,
        config,
        &format!("EHLO {}\r\n", config.ehlo_hostname),
    )
    .await?;
    let ehlo = read_reply_timeout(&mut reader, config).await?;
    if ehlo.code != 250 {
        return Err(DeliveryError::Protocol("EHLO rejected".into()));
    }
    if ehlo
        .lines
        .iter()
        .any(|line| line.eq_ignore_ascii_case("STARTTLS"))
    {
        send_command(&mut reader, config, "STARTTLS\r\n").await?;
        let starttls = read_reply_timeout(&mut reader, config).await?;
        if starttls.code != 220 {
            return Err(DeliveryError::Tls);
        }
        let transport = match reader.into_inner() {
            SmtpTransport::Plain(stream) => tls_connect(stream, host, config).await?,
            SmtpTransport::Tls(_) => return Err(DeliveryError::Tls),
        };
        io = SmtpTransport::Tls(Box::new(transport));
        reader = BufReader::new(io);
        send_command(
            &mut reader,
            config,
            &format!("EHLO {}\r\n", config.ehlo_hostname),
        )
        .await?;
        if read_reply_timeout(&mut reader, config).await?.code != 250 {
            return Err(DeliveryError::Protocol("EHLO after TLS rejected".into()));
        }
    } else if config.tls_policy == TlsPolicy::Required {
        return Err(DeliveryError::Tls);
    }
    send_command(
        &mut reader,
        config,
        &format!("MAIL FROM:<{}>\r\n", job.sender),
    )
    .await?;
    if !matches!(
        classify_status(read_reply_timeout(&mut reader, config).await?.code),
        DeliveryOutcome::DeliveredAccepted
    ) {
        return Err(DeliveryError::Protocol("MAIL FROM rejected".into()));
    }
    let mut accepted = 0;
    let mut rejected = 0;
    for recipient in &job.recipients {
        send_command(&mut reader, config, &format!("RCPT TO:<{}>\r\n", recipient)).await?;
        if classify_status(read_reply_timeout(&mut reader, config).await?.code).is_success() {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    if accepted == 0 {
        return Err(DeliveryError::PermanentFailure {
            code: Some(550),
            category: "all_recipients_rejected",
        });
    }
    send_command(&mut reader, config, "DATA\r\n").await?;
    if read_reply_timeout(&mut reader, config).await?.code != 354 {
        return Err(DeliveryError::Protocol("DATA rejected".into()));
    }
    write_message(&mut reader, &job.body, config.max_message_bytes).await?;
    let final_reply = read_reply_timeout(&mut reader, config).await?;
    let outcome = classify_status(final_reply.code);
    let report = DeliveryReport {
        outcome: outcome.clone(),
        accepted_recipients: accepted,
        rejected_recipients: rejected,
    };
    send_command(&mut reader, config, "QUIT\r\n").await.ok();
    if matches!(outcome, DeliveryOutcome::DeliveredAccepted) {
        Ok(report)
    } else if let DeliveryOutcome::PermanentFailure { code, category } = outcome {
        Err(DeliveryError::PermanentFailure { code, category })
    } else {
        Err(DeliveryError::Protocol(format!(
            "final SMTP status {}",
            final_reply.code
        )))
    }
}

async fn send_command(
    reader: &mut BufReader<SmtpTransport>,
    config: &DeliveryConfig,
    command: &str,
) -> Result<(), DeliveryError> {
    let writer = reader.get_mut();
    tokio::time::timeout(config.command_timeout, writer.write_all(command.as_bytes()))
        .await
        .map_err(|_| DeliveryError::ConnectTimeout)?
        .map_err(|_| DeliveryError::AmbiguousFinalResponse)
}

async fn read_reply_timeout(
    reader: &mut BufReader<SmtpTransport>,
    config: &DeliveryConfig,
) -> Result<SmtpReply, DeliveryError> {
    tokio::time::timeout(
        config.command_timeout,
        read_reply(reader, config.max_response_bytes),
    )
    .await
    .map_err(|_| DeliveryError::ConnectTimeout)?
}

async fn expect_code(
    reader: &mut BufReader<SmtpTransport>,
    config: &DeliveryConfig,
    expected: u16,
) -> Result<(), DeliveryError> {
    if read_reply_timeout(reader, config).await?.code == expected {
        Ok(())
    } else {
        Err(DeliveryError::Protocol("unexpected SMTP greeting".into()))
    }
}

async fn tls_connect(
    stream: TcpStream,
    host: &str,
    config: &DeliveryConfig,
) -> Result<TlsStream<TcpStream>, DeliveryError> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(tls_config));
    let name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|_| DeliveryError::Tls)?;
    let tls_stream: TlsStream<TcpStream> =
        tokio::time::timeout(config.tls_timeout, connector.connect(name, stream))
            .await
            .map_err(|_| DeliveryError::Tls)?
            .map_err(|_| DeliveryError::Tls)?;
    Ok(tls_stream)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;
    use mail_queue::model::ClaimedJob;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };
    use uuid::Uuid;

    use super::deliver;
    use crate::{config::DeliveryConfig, dns::DnsResolver, outcome::DeliveryOutcome};

    fn job(body: &[u8]) -> ClaimedJob {
        ClaimedJob {
            message_id: Uuid::new_v4(),
            sender: "sender@example.test".into(),
            recipients: vec!["recipient@example.test".into()],
            body: body.to_vec(),
            attempt_count: 1,
            lease_token: Uuid::new_v4(),
            lease_expires_at: Utc::now() + chrono::Duration::minutes(5),
        }
    }

    async fn fixture(response: &'static str, expect_data: bool) -> std::net::SocketAddr {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            write_half.write_all(b"220 fixture\r\n").await.unwrap();
            let mut line = String::new();
            for expected in ["EHLO", "MAIL FROM", "RCPT TO"] {
                line.clear();
                reader.read_line(&mut line).await.unwrap();
                assert!(line.starts_with(expected), "unexpected command: {line}");
                write_half.write_all(b"250 ok\r\n").await.unwrap();
            }
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "DATA\r\n");
            write_half.write_all(b"354 go\r\n").await.unwrap();
            let mut data = Vec::new();
            reader.read_until(b'\n', &mut data).await.unwrap();
            if expect_data {
                assert_eq!(&data, b"Subject: test\r\n");
                data.clear();
                reader.read_until(b'\n', &mut data).await.unwrap();
                assert_eq!(data, b"..leading-dot\r\n");
                data.clear();
                reader.read_until(b'\n', &mut data).await.unwrap();
                assert_eq!(data, b".\r\n");
            } else {
                assert_eq!(&data, b"body\r\n");
            }
            write_half.write_all(response.as_bytes()).await.unwrap();
        });
        address
    }

    fn config(endpoint: std::net::SocketAddr) -> DeliveryConfig {
        DeliveryConfig {
            enabled: true,
            test_endpoint: Some(endpoint),
            allow_loopback_test_destinations: true,
            require_public_destinations: true,
            command_timeout: Duration::from_secs(2),
            overall_timeout: Duration::from_secs(5),
            ..DeliveryConfig::default()
        }
    }

    #[tokio::test]
    async fn sends_bounded_dot_stuffed_message() {
        let endpoint = fixture("250 accepted\r\n", true).await;
        let report = deliver(
            &job(b"Subject: test\n.leading-dot\n"),
            &config(endpoint),
            &DnsResolver::default(),
        )
        .await
        .unwrap();
        assert_eq!(report.outcome, DeliveryOutcome::DeliveredAccepted);
    }

    #[tokio::test]
    async fn does_not_treat_temporary_final_response_as_success() {
        let endpoint = fixture("451 try later\r\n", false).await;
        let result = deliver(&job(b"body"), &config(endpoint), &DnsResolver::default()).await;
        assert!(result.is_err());
    }
}
