use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use base64::{Engine, engine::general_purpose::STANDARD};
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
    pub recipients: Vec<RecipientDeliveryReport>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecipientDeliveryStatus {
    AcceptedByRemote,
    TemporaryFailure,
    PermanentFailure,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipientDeliveryReport {
    pub recipient: String,
    pub status: RecipientDeliveryStatus,
    pub smtp_code: Option<u16>,
    pub diagnostic: String,
    pub remote_mx: Option<String>,
}

pub async fn deliver<R: Resolver>(
    job: &ClaimedJob,
    config: &DeliveryConfig,
    resolver: &R,
) -> Result<DeliveryReport, DeliveryError> {
    tokio::time::timeout(config.overall_timeout, deliver_inner(job, config, resolver))
        .await
        .map_err(|_| DeliveryError::AmbiguousFinalResponse)?
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
    validate_envelope_sender(&job.sender)?;
    if let Some(relay) = &config.smtp_relay {
        if !job.sender.eq_ignore_ascii_case(&relay.from_address) {
            return Err(DeliveryError::PermanentFailure {
                code: None,
                category: "relay_sender_mismatch",
            });
        }
        return deliver_via_relay(job, config, relay).await;
    }
    let mut recipients_by_domain = std::collections::BTreeMap::<String, Vec<String>>::new();
    for recipient in &job.recipients {
        let domain = recipient_domain(recipient)?;
        recipients_by_domain
            .entry(domain.trim_end_matches('.').to_ascii_lowercase())
            .or_default()
            .push(recipient.clone());
    }
    if recipients_by_domain.is_empty() {
        return Err(DeliveryError::NoCandidate);
    }
    let policy = DestinationPolicy {
        require_public: config.require_public_destinations,
        allow_loopback_test: config.allow_loopback_test_destinations,
    };
    let mut recipient_results = Vec::with_capacity(job.recipients.len());
    for (_, recipients) in recipients_by_domain {
        let domain = recipient_domain(&recipients[0])?;
        let normalized_domain = domain.trim_end_matches('.').to_ascii_lowercase();
        if !config
            .allowed_recipient_domains
            .iter()
            .any(|allowed| allowed == &normalized_domain)
        {
            let error = DeliveryError::PermanentFailure {
                code: None,
                category: "destination_not_allowlisted",
            };
            recipient_results.extend(recipients.into_iter().map(|recipient| {
                recipient_failure(
                    recipient,
                    RecipientDeliveryStatus::PermanentFailure,
                    &error,
                    None,
                )
            }));
            continue;
        }
        let mut domain_job = job.clone();
        domain_job.recipients = recipients.clone();
        match deliver_single_domain(&domain_job, domain, config, resolver, &policy).await {
            Ok(report) => recipient_results.extend(report.recipients),
            Err(error) => {
                let status = classify_delivery_error(&error);
                recipient_results.extend(
                    recipients.into_iter().map(|recipient| {
                        recipient_failure(recipient, status.clone(), &error, None)
                    }),
                );
            }
        }
    }
    let outcome = aggregate_outcome(&recipient_results);
    Ok(DeliveryReport {
        outcome,
        recipients: recipient_results,
    })
}

async fn deliver_via_relay(
    job: &ClaimedJob,
    config: &DeliveryConfig,
    relay: &crate::config::SmtpRelayConfig,
) -> Result<DeliveryReport, DeliveryError> {
    let stream = tokio::time::timeout(
        config.connect_timeout,
        TcpStream::connect((relay.host.as_str(), relay.port)),
    )
    .await
    .map_err(|_| DeliveryError::ConnectTimeout)?
    .map_err(|_| DeliveryError::ConnectTimeout)?;
    let mut transport = if relay.port == 465 {
        SmtpTransport::Tls(Box::new(tls_connect(stream, &relay.host, config).await?))
    } else {
        SmtpTransport::Plain(stream)
    };
    let mut reader = BufReader::new(transport);
    expect_code(&mut reader, config, 220).await?;
    send_command(
        &mut reader,
        config,
        &format!("EHLO {}\r\n", config.ehlo_hostname),
    )
    .await?;
    let ehlo = read_reply_timeout(&mut reader, config).await?;
    if ehlo.code != 250 {
        return Err(DeliveryError::Protocol("relay rejected EHLO".into()));
    }
    if relay.port == 587 {
        if !ehlo
            .lines
            .iter()
            .any(|line| line.eq_ignore_ascii_case("STARTTLS"))
        {
            return Err(DeliveryError::Tls);
        }
        send_command(&mut reader, config, "STARTTLS\r\n").await?;
        if read_reply_timeout(&mut reader, config).await?.code != 220 {
            return Err(DeliveryError::Tls);
        }
        let stream = match reader.into_inner() {
            SmtpTransport::Plain(stream) => tls_connect(stream, &relay.host, config).await?,
            SmtpTransport::Tls(_) => return Err(DeliveryError::Tls),
        };
        transport = SmtpTransport::Tls(Box::new(stream));
        reader = BufReader::new(transport);
        send_command(
            &mut reader,
            config,
            &format!("EHLO {}\r\n", config.ehlo_hostname),
        )
        .await?;
        if read_reply_timeout(&mut reader, config).await?.code != 250 {
            return Err(DeliveryError::Protocol(
                "relay rejected EHLO after TLS".into(),
            ));
        }
    }

    let credentials = STANDARD.encode(format!("\0{}\0{}", relay.username, relay.password));
    send_command(
        &mut reader,
        config,
        &format!("AUTH PLAIN {credentials}\r\n"),
    )
    .await?;
    let auth_reply = read_reply_timeout(&mut reader, config).await?;
    let auth_reply = if auth_reply.code == 334 {
        send_command(&mut reader, config, &format!("{credentials}\r\n")).await?;
        read_reply_timeout(&mut reader, config).await?
    } else {
        auth_reply
    };
    if auth_reply.code != 235 {
        return Err(status_error(auth_reply.code, "AUTH"));
    }

    send_command(
        &mut reader,
        config,
        &format!("MAIL FROM:<{}>\r\n", job.sender),
    )
    .await?;
    let mail_status = read_reply_timeout(&mut reader, config).await?.code;
    if !classify_status(mail_status).is_success() {
        return Err(status_error(mail_status, "MAIL FROM"));
    }
    let mut accepted = Vec::new();
    let mut recipient_results = vec![None; job.recipients.len()];
    for (index, recipient) in job.recipients.iter().enumerate() {
        send_command(&mut reader, config, &format!("RCPT TO:<{}>\r\n", recipient)).await?;
        let reply = read_reply_timeout(&mut reader, config).await?;
        if classify_status(reply.code).is_success() {
            accepted.push((index, recipient.clone()));
        } else {
            recipient_results[index] = Some(recipient_report(
                recipient.clone(),
                reply.code,
                reply.lines.join(" "),
                &relay.host,
            ));
        }
    }
    if accepted.is_empty() {
        let recipient_results = recipient_results
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DeliveryError::InternalFailure)?;
        return Ok(DeliveryReport {
            outcome: aggregate_outcome(&recipient_results),
            recipients: recipient_results,
        });
    }
    send_command(&mut reader, config, "DATA\r\n").await?;
    let data_reply = read_reply_timeout(&mut reader, config).await?;
    if data_reply.code != 354 {
        for (index, recipient) in accepted {
            recipient_results[index] = Some(recipient_report(
                recipient,
                data_reply.code,
                data_reply.lines.join(" "),
                &relay.host,
            ));
        }
        let recipient_results = recipient_results
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DeliveryError::InternalFailure)?;
        return Ok(DeliveryReport {
            outcome: aggregate_outcome(&recipient_results),
            recipients: recipient_results,
        });
    }
    let write_result = write_message(&mut reader, &job.body, config.max_message_bytes).await;
    let final_reply = match write_result {
        Ok(()) => tokio::time::timeout(
            config.command_timeout,
            read_reply(&mut reader, config.max_response_bytes),
        )
        .await
        .ok()
        .and_then(Result::ok),
        Err(_) => None,
    };
    for (index, recipient) in accepted {
        recipient_results[index] = Some(match &final_reply {
            Some(reply) => {
                recipient_report(recipient, reply.code, reply.lines.join(" "), &relay.host)
            }
            None => RecipientDeliveryReport {
                recipient,
                status: RecipientDeliveryStatus::Uncertain,
                smtp_code: None,
                diagnostic:
                    "connection lost during message submission; remote acceptance is unknown".into(),
                remote_mx: Some(relay.host.clone()),
            },
        });
    }
    let recipient_results = recipient_results
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(DeliveryError::InternalFailure)?;
    let report = DeliveryReport {
        outcome: aggregate_outcome(&recipient_results),
        recipients: recipient_results,
    };
    send_command(&mut reader, config, "QUIT\r\n").await.ok();
    Ok(report)
}

async fn deliver_single_domain<R: Resolver>(
    job: &ClaimedJob,
    domain: &str,
    config: &DeliveryConfig,
    resolver: &R,
    policy: &DestinationPolicy,
) -> Result<DeliveryReport, DeliveryError> {
    let candidates = if let Some(endpoint) = config.test_endpoint {
        if !policy.allows(endpoint.ip()) {
            return Err(DeliveryError::UnsafeDestination);
        }
        vec![(endpoint, "localhost".to_owned())]
    } else {
        let mx = tokio::time::timeout(
            config.dns_timeout,
            resolver.resolve_mx(domain, config.dns_timeout, config.max_mx_candidates),
        )
        .await
        .map_err(|_| DeliveryError::Dns)??;
        mx.into_iter()
            .flat_map(|candidate| {
                socket_candidates(&candidate, policy)
                    .into_iter()
                    .map(move |address| (address, candidate.host.clone()))
            })
            .take(config.max_fallback_attempts)
            .collect::<Vec<_>>()
    };
    if candidates.is_empty() {
        return Err(DeliveryError::UnsafeDestination);
    }
    let mut last_error = DeliveryError::NoCandidate;
    for (address, host) in candidates {
        if !policy.allows(address.ip()) {
            last_error = DeliveryError::UnsafeDestination;
            continue;
        }
        match deliver_to(job, config, address, &host).await {
            Ok(report) => return Ok(report),
            Err(error) if should_try_next_mx(&error) => last_error = error,
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

fn validate_envelope_sender(sender: &str) -> Result<(), DeliveryError> {
    if sender.is_empty() {
        return Ok(());
    }
    recipient_domain(sender).map(|_| ())
}

fn should_try_next_mx(error: &DeliveryError) -> bool {
    matches!(
        error,
        DeliveryError::ConnectTimeout | DeliveryError::Protocol(_)
    )
}

pub(crate) fn classify_delivery_error(error: &DeliveryError) -> RecipientDeliveryStatus {
    match error {
        DeliveryError::PermanentFailure { .. }
        | DeliveryError::NullMx
        | DeliveryError::UnsafeDestination
        | DeliveryError::Tls
        | DeliveryError::MessageTooLarge
        | DeliveryError::Disabled
        | DeliveryError::MultiDomainUnsupported => RecipientDeliveryStatus::PermanentFailure,
        DeliveryError::AmbiguousFinalResponse => RecipientDeliveryStatus::Uncertain,
        _ => RecipientDeliveryStatus::TemporaryFailure,
    }
}

pub(crate) fn recipient_failure(
    recipient: String,
    status: RecipientDeliveryStatus,
    error: &DeliveryError,
    remote_mx: Option<String>,
) -> RecipientDeliveryReport {
    let smtp_code = match error {
        DeliveryError::TemporaryFailure { code }
        | DeliveryError::PermanentFailure {
            code: Some(code), ..
        } => Some(*code),
        _ => None,
    };
    RecipientDeliveryReport {
        recipient,
        status,
        smtp_code,
        diagnostic: safe_diagnostic(&error.to_string()),
        remote_mx,
    }
}

fn aggregate_outcome(results: &[RecipientDeliveryReport]) -> DeliveryOutcome {
    let accepted = results
        .iter()
        .filter(|result| result.status == RecipientDeliveryStatus::AcceptedByRemote)
        .count();
    if accepted == results.len() && accepted > 0 {
        DeliveryOutcome::DeliveredAccepted
    } else if accepted > 0 {
        DeliveryOutcome::PartiallyAccepted
    } else if results
        .iter()
        .any(|result| result.status == RecipientDeliveryStatus::Uncertain)
    {
        DeliveryOutcome::Uncertain
    } else if results
        .iter()
        .any(|result| result.status == RecipientDeliveryStatus::TemporaryFailure)
    {
        DeliveryOutcome::TemporaryFailure {
            code: None,
            category: "temporary_delivery",
        }
    } else {
        DeliveryOutcome::PermanentFailure {
            code: None,
            category: "permanent_delivery",
        }
    }
}

fn safe_diagnostic(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(512)
        .collect()
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
    let mail_status = read_reply_timeout(&mut reader, config).await?.code;
    if !classify_status(mail_status).is_success() {
        return Err(status_error(mail_status, "MAIL FROM"));
    }
    let mut accepted = Vec::new();
    let mut recipient_results = vec![None; job.recipients.len()];
    for (index, recipient) in job.recipients.iter().enumerate() {
        send_command(&mut reader, config, &format!("RCPT TO:<{}>\r\n", recipient)).await?;
        let reply = read_reply_timeout(&mut reader, config).await?;
        if classify_status(reply.code).is_success() {
            accepted.push((index, recipient.clone()));
        } else {
            recipient_results[index] = Some(recipient_report(
                recipient.clone(),
                reply.code,
                reply.lines.join(" "),
                host,
            ));
        }
    }
    if accepted.is_empty() {
        let _ = send_command(&mut reader, config, "RSET\r\n").await;
        let recipient_results = recipient_results
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DeliveryError::InternalFailure)?;
        let outcome = aggregate_outcome(&recipient_results);
        return Ok(DeliveryReport {
            outcome,
            recipients: recipient_results,
        });
    }
    send_command(&mut reader, config, "DATA\r\n").await?;
    let data_reply = read_reply_timeout(&mut reader, config).await?;
    if data_reply.code != 354 {
        for (index, recipient) in accepted {
            recipient_results[index] = Some(recipient_report(
                recipient,
                data_reply.code,
                data_reply.lines.join(" "),
                host,
            ));
        }
        let recipient_results = recipient_results
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DeliveryError::InternalFailure)?;
        return Ok(DeliveryReport {
            outcome: aggregate_outcome(&recipient_results),
            recipients: recipient_results,
        });
    }
    let write_result = write_message(&mut reader, &job.body, config.max_message_bytes).await;
    let final_reply = match write_result {
        Ok(()) => tokio::time::timeout(
            config.command_timeout,
            read_reply(&mut reader, config.max_response_bytes),
        )
        .await
        .ok()
        .and_then(Result::ok),
        Err(_) => None,
    };
    for (index, recipient) in accepted {
        recipient_results[index] = Some(match &final_reply {
            Some(reply) => recipient_report(recipient, reply.code, reply.lines.join(" "), host),
            None => RecipientDeliveryReport {
                recipient,
                status: RecipientDeliveryStatus::Uncertain,
                smtp_code: None,
                diagnostic:
                    "connection lost during message submission; remote acceptance is unknown".into(),
                remote_mx: Some(host.to_owned()),
            },
        });
    }
    let recipient_results = recipient_results
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(DeliveryError::InternalFailure)?;
    let outcome = aggregate_outcome(&recipient_results);
    let report = DeliveryReport {
        outcome,
        recipients: recipient_results,
    };
    send_command(&mut reader, config, "QUIT\r\n").await.ok();
    Ok(report)
}

fn recipient_report(
    recipient: String,
    code: u16,
    diagnostic: String,
    remote_mx: &str,
) -> RecipientDeliveryReport {
    let status = match classify_status(code) {
        DeliveryOutcome::DeliveredAccepted => RecipientDeliveryStatus::AcceptedByRemote,
        DeliveryOutcome::TemporaryFailure { .. } => RecipientDeliveryStatus::TemporaryFailure,
        DeliveryOutcome::PermanentFailure { .. } => RecipientDeliveryStatus::PermanentFailure,
        _ => RecipientDeliveryStatus::PermanentFailure,
    };
    RecipientDeliveryReport {
        recipient,
        status,
        smtp_code: Some(code),
        diagnostic: safe_diagnostic(&diagnostic),
        remote_mx: Some(remote_mx.to_owned()),
    }
}

fn status_error(code: u16, phase: &'static str) -> DeliveryError {
    match classify_status(code) {
        DeliveryOutcome::TemporaryFailure { .. } => DeliveryError::TemporaryFailure { code },
        DeliveryOutcome::PermanentFailure { category, .. } => DeliveryError::PermanentFailure {
            code: Some(code),
            category,
        },
        _ => DeliveryError::Protocol(format!("unexpected SMTP status {code} during {phase}")),
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
        .map_err(|_| DeliveryError::Protocol("SMTP command write timed out".into()))?
        .map_err(|_| DeliveryError::Protocol("SMTP command write failed".into()))
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
            allowed_recipient_domains: vec!["example.test".into(), "example.invalid".into()],
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
    async fn records_temporary_final_response_per_recipient() {
        let endpoint = fixture("451 try later\r\n", false).await;
        let report = deliver(&job(b"body"), &config(endpoint), &DnsResolver::default())
            .await
            .unwrap();
        assert_eq!(
            report.outcome,
            DeliveryOutcome::TemporaryFailure {
                code: None,
                category: "temporary_delivery"
            }
        );
        assert_eq!(
            report.recipients[0].status,
            super::RecipientDeliveryStatus::TemporaryFailure
        );
        assert_eq!(report.recipients[0].smtp_code, Some(451));
    }

    #[tokio::test]
    async fn lost_final_response_is_uncertain_and_never_reported_as_accepted() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
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
            loop {
                line.clear();
                reader.read_line(&mut line).await.unwrap();
                if line == ".\r\n" {
                    break;
                }
            }
        });

        let report = deliver(&job(b"body"), &config(endpoint), &DnsResolver::default())
            .await
            .unwrap();
        assert_eq!(report.outcome, DeliveryOutcome::Uncertain);
        assert_eq!(
            report.recipients[0].status,
            super::RecipientDeliveryStatus::Uncertain
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn routes_mixed_domain_batches_independently() {
        let mut batch = job(b"Subject: test\n.leading-dot\n");
        batch.recipients.push("other@example.invalid".into());
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
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
                loop {
                    line.clear();
                    reader.read_line(&mut line).await.unwrap();
                    if line == ".\r\n" {
                        break;
                    }
                }
                write_half.write_all(b"250 accepted\r\n").await.unwrap();
            }
        });
        let report = deliver(&batch, &config(endpoint), &DnsResolver::default())
            .await
            .unwrap();
        assert_eq!(report.outcome, DeliveryOutcome::DeliveredAccepted);
        assert_eq!(report.recipients.len(), 2);
        assert!(
            report
                .recipients
                .iter()
                .all(|recipient| recipient.status
                    == super::RecipientDeliveryStatus::AcceptedByRemote)
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn submits_only_recipients_accepted_by_the_remote_server() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            write_half.write_all(b"220 fixture\r\n").await.unwrap();
            let mut line = String::new();
            for expected in ["EHLO", "MAIL FROM", "RCPT TO", "RCPT TO"] {
                line.clear();
                reader.read_line(&mut line).await.unwrap();
                assert!(line.starts_with(expected), "unexpected command: {line}");
                let reply = if expected == "RCPT TO" && line.contains("rejected@") {
                    b"550 no such recipient\r\n".as_slice()
                } else {
                    b"250 ok\r\n".as_slice()
                };
                write_half.write_all(reply).await.unwrap();
            }
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "DATA\r\n");
            write_half.write_all(b"354 go\r\n").await.unwrap();
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "body\r\n");
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, ".\r\n");
            write_half.write_all(b"250 accepted\r\n").await.unwrap();
        });

        let mut batch = job(b"body");
        batch.recipients = vec![
            "accepted@example.test".into(),
            "rejected@example.test".into(),
        ];
        let report = deliver(&batch, &config(endpoint), &DnsResolver::default())
            .await
            .unwrap();
        assert_eq!(report.outcome, DeliveryOutcome::PartiallyAccepted);
        assert_eq!(report.recipients.len(), 2);
        assert_eq!(
            report.recipients[0].status,
            super::RecipientDeliveryStatus::AcceptedByRemote
        );
        assert_eq!(
            report.recipients[1].status,
            super::RecipientDeliveryStatus::PermanentFailure
        );
        server.await.unwrap();
    }
}
