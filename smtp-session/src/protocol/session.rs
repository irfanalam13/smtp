use std::{io, net::IpAddr};

use base64::{Engine, engine::general_purpose::STANDARD};
use tokio::{
    io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    time::timeout,
};

use crate::auth::{AuthenticatedIdentity, AuthenticationError};
use crate::inbound::{InboundContext, InboundError};
use crate::{
    command::{self, Command, ParseError},
    config::Config,
    error::SmtpError,
    message::Message,
    reply::Reply,
};
use mail_queue::EnqueueRequest;
use shared_types::{MessageEnvelope, MimeError, MimeLimits, parse_canonical_message};
use std::sync::atomic::Ordering;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Connected,
    Greeted,
    Authenticated,
    MailFromAccepted,
    RecipientsAccepted,
    Closed,
}

pub struct SessionRunResult {
    pub messages: Vec<Message>,
    pub disposition: SessionDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionDisposition {
    Closed,
    StartTls,
}

pub struct Session {
    config: Config,
    state: State,
    sender: Option<String>,
    recipients: Vec<String>,
    helo_domain: String,
    authenticated_identity: Option<AuthenticatedIdentity>,
    tls_active: bool,
    peer_ip: IpAddr,
    esmtp_enabled: bool,
    transaction_size_limit: usize,
    declared_message_size: Option<usize>,
    transaction_body_8bit: bool,
    capture_messages: bool,
}

impl Session {
    pub fn new(config: Config) -> Self {
        Self::new_with_transport(config, false)
    }

    pub fn new_with_transport(config: Config, tls_active: bool) -> Self {
        Self::new_with_peer(
            config,
            tls_active,
            IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        )
    }

    pub fn new_with_peer(config: Config, tls_active: bool, peer_ip: IpAddr) -> Self {
        Self::new_with_peer_capture(config, tls_active, peer_ip, true)
    }

    pub(crate) fn new_for_server(config: Config, tls_active: bool, peer_ip: IpAddr) -> Self {
        Self::new_with_peer_capture(config, tls_active, peer_ip, false)
    }

    fn new_with_peer_capture(
        config: Config,
        tls_active: bool,
        peer_ip: IpAddr,
        capture_messages: bool,
    ) -> Self {
        let transaction_size_limit = config.max_message_bytes;
        let helo_domain = config.server_name.clone();
        Self {
            config,
            state: State::Connected,
            sender: None,
            recipients: Vec::new(),
            helo_domain,
            authenticated_identity: None,
            tls_active,
            peer_ip,
            esmtp_enabled: false,
            transaction_size_limit,
            declared_message_size: None,
            transaction_body_8bit: false,
            capture_messages,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn process(&mut self, command: Command) -> Reply {
        match command {
            Command::Ehlo(domain) => {
                self.reset_transaction();
                self.esmtp_enabled = true;
                self.helo_domain = domain;
                let mut capabilities = vec![
                    self.config.server_name.clone(),
                    format!("SIZE {}", self.config.max_message_bytes),
                ];
                if self.config.enable_8bitmime {
                    capabilities.push("8BITMIME".into());
                }
                if self.config.authenticator.is_some() && self.tls_active {
                    capabilities.push("AUTH PLAIN".into());
                }
                if self.config.tls.is_some() && !self.tls_active {
                    capabilities.push("STARTTLS".into());
                }
                Reply::multiline(250, capabilities)
            }
            Command::Helo(domain) => {
                self.reset_transaction();
                self.esmtp_enabled = false;
                self.helo_domain = domain;
                Reply::new(250, self.config.server_name.clone())
            }
            Command::MailFrom {
                sender,
                size,
                body_8bit,
                has_body_parameter,
                has_esmtp_parameters,
            } if matches!(self.state, State::Greeted | State::Authenticated)
                && (!self.config.require_authentication
                    || self.authenticated_identity.is_some()) =>
            {
                if has_esmtp_parameters && !self.esmtp_enabled {
                    return Reply::new(555, "ESMTP parameters require EHLO");
                }
                if has_body_parameter && !self.config.enable_8bitmime {
                    return Reply::new(555, "8BITMIME parameter not supported");
                }
                if size.is_some_and(|size| size > self.transaction_size_limit) {
                    return Reply::new(552, "message size exceeds fixed maximum");
                }
                self.sender = Some(sender);
                self.recipients.clear();
                self.declared_message_size = size;
                self.transaction_body_8bit = body_8bit;
                self.state = State::MailFromAccepted;
                Reply::new(250, "sender ok")
            }
            Command::MailFrom { .. }
                if self.config.require_authentication
                    && self.authenticated_identity.is_none()
                    && matches!(self.state, State::Greeted | State::Authenticated) =>
            {
                Reply::new(530, "authentication required")
            }
            Command::RcptTo(recipient)
                if matches!(
                    self.state,
                    State::MailFromAccepted | State::RecipientsAccepted
                ) =>
            {
                if self.recipients.len() >= self.config.max_recipients {
                    return Reply::new(452, "too many recipients");
                }
                self.recipients.push(recipient);
                self.state = State::RecipientsAccepted;
                Reply::new(250, "recipient ok")
            }
            Command::Data if self.state == State::RecipientsAccepted => {
                Reply::new(354, "end data with <CRLF>.<CRLF>")
            }
            Command::Rset if self.state != State::Closed => {
                self.reset_transaction();
                Reply::new(250, "reset")
            }
            Command::Noop if self.state != State::Closed => Reply::new(250, "ok"),
            Command::Quit => {
                self.state = State::Closed;
                Reply::new(221, "closing connection")
            }
            Command::Auth { .. } => {
                if self.config.require_tls_for_auth && !self.tls_active {
                    Reply::new(538, "encryption required for authentication")
                } else if self.config.authenticator.is_none() {
                    Reply::new(502, "authentication not available")
                } else {
                    Reply::new(503, "authentication exchange required")
                }
            }
            Command::StartTls => Reply::new(502, "command not implemented"),
            Command::Unsupported(_) => Reply::new(502, "command not implemented"),
            _ => Reply::new(503, "bad sequence of commands"),
        }
    }

    pub async fn run<R, W>(
        &mut self,
        reader: &mut BufReader<R>,
        writer: &mut W,
    ) -> Result<SessionRunResult, SmtpError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        self.run_inner(reader, writer, true).await
    }

    pub async fn resume_after_tls<R, W>(
        &mut self,
        reader: &mut BufReader<R>,
        writer: &mut W,
    ) -> Result<SessionRunResult, SmtpError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        self.run_inner(reader, writer, false).await
    }

    async fn run_inner<R, W>(
        &mut self,
        reader: &mut BufReader<R>,
        writer: &mut W,
        send_greeting: bool,
    ) -> Result<SessionRunResult, SmtpError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        if send_greeting {
            write_reply(writer, Reply::new(220, "smtp-test ready")).await?;
        }
        let mut buffer = Vec::with_capacity(self.config.max_command_length.min(256));
        let mut messages = Vec::new();
        loop {
            buffer.clear();
            let read = match timeout(
                self.config.command_timeout,
                read_bounded(
                    reader,
                    &mut buffer,
                    self.config.max_command_length,
                    self.config.max_buffered_input,
                ),
            )
            .await
            {
                Err(_) => return Err(SmtpError::Timeout),
                Ok(Err(SmtpError::LineTooLong)) => {
                    write_reply(writer, Reply::new(500, "line too long")).await?;
                    continue;
                }
                Ok(Err(error)) => return Err(error),
                Ok(Ok(read)) => read,
            };
            if read == 0 {
                return Ok(SessionRunResult {
                    messages,
                    disposition: SessionDisposition::Closed,
                });
            }
            if !buffer.ends_with(b"\r\n") {
                write_reply(writer, Reply::new(500, "line must end with CRLF")).await?;
                continue;
            }
            buffer.truncate(buffer.len() - 2);
            let command = match command::parse(&buffer) {
                Ok(command) => command,
                Err(error) => {
                    write_reply(writer, parse_error_reply(error)).await?;
                    continue;
                }
            };
            if matches!(command, Command::StartTls) {
                if self.config.tls.is_none() || self.tls_active {
                    write_reply(writer, Reply::new(502, "STARTTLS not available")).await?;
                    continue;
                }
                if self.state != State::Greeted {
                    write_reply(writer, Reply::new(503, "send EHLO before STARTTLS")).await?;
                    continue;
                }
                if !reader.buffer().is_empty() {
                    write_reply(
                        writer,
                        Reply::new(554, "plaintext data buffered after STARTTLS"),
                    )
                    .await?;
                    return Ok(SessionRunResult {
                        messages,
                        disposition: SessionDisposition::Closed,
                    });
                }
                write_reply(writer, Reply::new(220, "ready to start TLS")).await?;
                return Ok(SessionRunResult {
                    messages,
                    disposition: SessionDisposition::StartTls,
                });
            }
            let is_data =
                matches!(command, Command::Data) && self.state == State::RecipientsAccepted;
            if let Command::Auth {
                mechanism,
                initial_response,
            } = command
            {
                self.authenticate(&mechanism, initial_response, reader, writer)
                    .await?;
                continue;
            }
            if matches!(self.state, State::Greeted | State::Authenticated)
                && let Command::MailFrom { sender, .. } = &command
                && !sender.is_empty()
                && let Some(identity) = &self.authenticated_identity
            {
                let authorizer = self
                    .config
                    .sender_authorizer
                    .as_ref()
                    .expect("validated SMTP AUTH sender authorizer");
                match authorizer
                    .authorize_sender(identity.tenant_id, sender)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => {
                        self.config
                            .metrics
                            .rejected_messages
                            .fetch_add(1, Ordering::Relaxed);
                        write_reply(writer, Reply::new(550, "sender domain not authorized"))
                            .await?;
                        continue;
                    }
                    Err(_) => {
                        eprintln!("smtp sender authorization backend failure: database");
                        self.config
                            .metrics
                            .temporary_failures
                            .fetch_add(1, Ordering::Relaxed);
                        write_reply(
                            writer,
                            Reply::new(451, "temporary sender authorization failure"),
                        )
                        .await?;
                        continue;
                    }
                }
            }
            if matches!(self.state, State::Greeted | State::Authenticated)
                && let Command::MailFrom { sender, .. } = &command
            {
                self.transaction_size_limit = self.config.max_message_bytes;
                if let Some(identity) = &self.authenticated_identity {
                    let authorizer = self
                        .config
                        .sender_authorizer
                        .as_ref()
                        .expect("validated SMTP AUTH sender authorizer");
                    match authorizer
                        .max_message_bytes(identity.tenant_id, sender)
                        .await
                    {
                        Ok(Some(limit)) => {
                            self.transaction_size_limit = self.transaction_size_limit.min(limit);
                        }
                        Ok(None) => {}
                        Err(_) => {
                            self.config
                                .metrics
                                .temporary_failures
                                .fetch_add(1, Ordering::Relaxed);
                            write_reply(
                                writer,
                                Reply::new(451, "temporary message policy failure"),
                            )
                            .await?;
                            continue;
                        }
                    }
                }
            }
            if is_data && let Some(inbound) = &self.config.inbound {
                match inbound
                    .reserve_rate_capacity(self.peer_ip, &self.recipients)
                    .await
                {
                    Ok(()) => {}
                    Err(InboundError::Rejected | InboundError::InvalidRecipient) => {
                        self.config
                            .metrics
                            .rejected_messages
                            .fetch_add(1, Ordering::Relaxed);
                        write_reply(writer, Reply::new(550, "recipient not accepted")).await?;
                        self.reset_transaction();
                        continue;
                    }
                    Err(InboundError::RateLimited) => {
                        self.config
                            .metrics
                            .inbound_rate_limited
                            .fetch_add(1, Ordering::Relaxed);
                        self.config
                            .metrics
                            .temporary_failures
                            .fetch_add(1, Ordering::Relaxed);
                        write_reply(writer, Reply::new(451, "inbound rate limit exceeded")).await?;
                        self.reset_transaction();
                        continue;
                    }
                    Err(_) => {
                        self.config
                            .metrics
                            .temporary_failures
                            .fetch_add(1, Ordering::Relaxed);
                        write_reply(writer, Reply::new(451, "temporary inbound policy failure"))
                            .await?;
                        continue;
                    }
                }
                match inbound.max_message_bytes(&self.recipients).await {
                    Ok(Some(limit)) => {
                        self.transaction_size_limit = self.transaction_size_limit.min(limit);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        self.config
                            .metrics
                            .temporary_failures
                            .fetch_add(1, Ordering::Relaxed);
                        write_reply(writer, Reply::new(451, "temporary message policy failure"))
                            .await?;
                        continue;
                    }
                }
                if self
                    .declared_message_size
                    .is_some_and(|size| size > self.transaction_size_limit)
                {
                    write_reply(
                        writer,
                        Reply::new(552, "message size exceeds recipient limit"),
                    )
                    .await?;
                    self.reset_transaction();
                    continue;
                }
            }
            let reply = self.process(command);
            write_reply(writer, reply).await?;
            if self.state == State::Closed {
                return Ok(SessionRunResult {
                    messages,
                    disposition: SessionDisposition::Closed,
                });
            }
            if is_data {
                let body = match self.read_data(reader).await {
                    Ok(body) => body,
                    Err(SmtpError::MessageTooLarge) => {
                        write_reply(writer, Reply::new(552, "message too large")).await?;
                        self.reset_transaction();
                        continue;
                    }
                    Err(SmtpError::LineTooLong) => {
                        write_reply(writer, Reply::new(552, "message line too long")).await?;
                        self.reset_transaction();
                        continue;
                    }
                    Err(SmtpError::EightBitMimeRequired) => {
                        write_reply(writer, Reply::new(554, "8-bit body requires BODY=8BITMIME"))
                            .await?;
                        self.reset_transaction();
                        continue;
                    }
                    Err(SmtpError::InvalidHeaderEncoding) => {
                        write_reply(
                            writer,
                            Reply::new(554, "non-ASCII headers require SMTPUTF8"),
                        )
                        .await?;
                        self.reset_transaction();
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                let canonical = match parse_canonical_message(
                    MessageEnvelope {
                        sender: self.sender.clone().expect("DATA requires sender"),
                        recipients: self.recipients.clone(),
                    },
                    body,
                    MimeLimits {
                        max_message_bytes: self.transaction_size_limit,
                        ..MimeLimits::default()
                    },
                )
                .await
                {
                    Ok(canonical) => canonical,
                    Err(error) => {
                        let reply = match &error {
                            MimeError::MessageTooLarge
                            | MimeError::HeadersTooLarge
                            | MimeError::TooManyHeaders
                            | MimeError::TooManyParts
                            | MimeError::TooManyAttachments
                            | MimeError::DecodedContentTooLarge
                            | MimeError::ExpansionRatioExceeded
                            | MimeError::IndexedMetadataTooLarge => {
                                Reply::new(552, "message exceeds canonical parsing limits")
                            }
                            MimeError::InvalidMessage => {
                                Reply::new(554, "message content is not parseable within limits")
                            }
                            MimeError::ParseTimedOut | MimeError::ParseWorkerFailed(_) => {
                                Reply::new(451, "temporary message parsing timeout")
                            }
                        };
                        write_reply(writer, reply).await?;
                        self.reset_transaction();
                        if let MimeError::ParseWorkerFailed(detail) = error {
                            return Err(SmtpError::Persistence(detail));
                        }
                        continue;
                    }
                };
                self.config
                    .metrics
                    .message_bytes
                    .fetch_add(canonical.raw_message().len() as u64, Ordering::Relaxed);
                if self.config.inbound.is_none()
                    && let Some(queue) = &self.config.queue
                {
                    let request = EnqueueRequest::new(
                        &canonical,
                        Uuid::new_v4().to_string(),
                        "smtp-loopback".to_string(),
                    );
                    if queue.enqueue(&request).await.is_err() {
                        write_reply(
                            writer,
                            Reply::new(451, "temporary local persistence failure"),
                        )
                        .await?;
                        self.reset_transaction();
                        self.config
                            .metrics
                            .temporary_failures
                            .fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                }
                if let Some(inbound) = &self.config.inbound
                    && let Err(error) = inbound
                        .accept(
                            &canonical,
                            &InboundContext {
                                peer_ip: self.peer_ip,
                                helo_domain: self.helo_domain.clone(),
                                received_by: self.config.server_name.clone(),
                                received_over_tls: self.tls_active,
                            },
                        )
                        .await
                {
                    let temporary = matches!(
                        &error,
                        InboundError::Temporary
                            | InboundError::RateLimited
                            | InboundError::InvalidRateLimitConfiguration
                    );
                    let reply = match error {
                        InboundError::Rejected | InboundError::InvalidRecipient => {
                            Reply::new(550, "recipient not accepted")
                        }
                        InboundError::TooLarge => Reply::new(552, "message too large"),
                        InboundError::QuotaExceeded => {
                            self.config
                                .metrics
                                .inbound_quota_rejections
                                .fetch_add(1, Ordering::Relaxed);
                            Reply::new(552, "mailbox storage quota exceeded")
                        }
                        InboundError::MalwareDetected => {
                            self.config
                                .metrics
                                .inbound_malware_rejections
                                .fetch_add(1, Ordering::Relaxed);
                            Reply::new(554, "message rejected by local security policy")
                        }
                        InboundError::SpamDetected => {
                            self.config
                                .metrics
                                .inbound_spam_rejections
                                .fetch_add(1, Ordering::Relaxed);
                            Reply::new(554, "message rejected by local security policy")
                        }
                        InboundError::AuthenticationRejected => {
                            self.config
                                .metrics
                                .inbound_auth_rejections
                                .fetch_add(1, Ordering::Relaxed);
                            Reply::new(554, "message rejected by sender authentication policy")
                        }
                        InboundError::RateLimited => {
                            self.config
                                .metrics
                                .inbound_rate_limited
                                .fetch_add(1, Ordering::Relaxed);
                            Reply::new(451, "inbound rate limit exceeded")
                        }
                        InboundError::InvalidRateLimitConfiguration => {
                            Reply::new(451, "temporary inbound policy failure")
                        }
                        InboundError::InvalidMessage => {
                            Reply::new(554, "message metadata is invalid")
                        }
                        InboundError::Temporary => {
                            Reply::new(451, "temporary local persistence failure")
                        }
                    };
                    write_reply(writer, reply).await?;
                    self.reset_transaction();
                    if temporary {
                        self.config
                            .metrics
                            .temporary_failures
                            .fetch_add(1, Ordering::Relaxed);
                    } else {
                        self.config
                            .metrics
                            .rejected_messages
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    continue;
                }
                if self.capture_messages {
                    messages.push(Message {
                        sender: canonical.envelope().sender.clone(),
                        recipients: canonical.envelope().recipients.clone(),
                        body: canonical.raw_message().to_vec(),
                    });
                }
                self.config
                    .metrics
                    .accepted_messages
                    .fetch_add(1, Ordering::Relaxed);
                self.reset_transaction();
                write_reply(
                    writer,
                    Reply::new(
                        250,
                        if self.config.inbound.is_some() {
                            "message accepted for durable inbound storage"
                        } else if self.config.queue.is_some() {
                            "message queued for test processing only"
                        } else {
                            "message accepted for test use only"
                        },
                    ),
                )
                .await?;
            }
        }
    }

    async fn authenticate(
        &mut self,
        mechanism: &str,
        response: Option<String>,
        reader: &mut (impl AsyncBufRead + Unpin),
        writer: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), SmtpError> {
        self.config
            .metrics
            .authentication_attempts
            .fetch_add(1, Ordering::Relaxed);
        let Some(authenticator) = self.config.authenticator.clone() else {
            write_reply(writer, Reply::new(502, "authentication not available")).await?;
            return Ok(());
        };
        if self.config.require_tls_for_auth && !self.tls_active {
            if self.record_auth_failure("plaintext-auth").await.is_err() {
                self.config
                    .metrics
                    .authentication_backend_failures
                    .fetch_add(1, Ordering::Relaxed);
            }
            self.config
                .metrics
                .authentication_failures
                .fetch_add(1, Ordering::Relaxed);
            write_reply(
                writer,
                Reply::new(538, "encryption required for authentication"),
            )
            .await?;
            return Ok(());
        }
        if self.state != State::Greeted {
            write_reply(writer, Reply::new(503, "send EHLO before AUTH")).await?;
            return Ok(());
        }
        let ip_limited = match self.is_ip_limited().await {
            Ok(limited) => limited,
            Err(error) => {
                self.log_rate_limit_error(&error);
                self.config
                    .metrics
                    .authentication_backend_failures
                    .fetch_add(1, Ordering::Relaxed);
                write_reply(
                    writer,
                    Reply::new(454, "authentication temporarily unavailable"),
                )
                .await?;
                return Ok(());
            }
        };
        if ip_limited {
            self.config
                .metrics
                .authentication_throttled
                .fetch_add(1, Ordering::Relaxed);
            write_reply(
                writer,
                Reply::new(454, "authentication temporarily unavailable"),
            )
            .await?;
            return Ok(());
        }
        if self.authenticated_identity.is_some()
            || self.state == State::MailFromAccepted
            || self.state == State::RecipientsAccepted
        {
            write_reply(writer, Reply::new(503, "authentication not permitted")).await?;
            return Ok(());
        }
        if mechanism != "PLAIN" {
            write_reply(
                writer,
                Reply::new(504, "unsupported authentication mechanism"),
            )
            .await?;
            return Ok(());
        }
        let response = match response {
            Some(response) => response,
            None => {
                write_reply(writer, Reply::new(334, "")).await?;
                let mut line = Vec::new();
                read_bounded(
                    reader,
                    &mut line,
                    self.config.max_command_length,
                    self.config.max_buffered_input,
                )
                .await?;
                if !line.ends_with(b"\r\n") {
                    write_reply(writer, Reply::new(501, "invalid authentication response")).await?;
                    return Ok(());
                }
                String::from_utf8_lossy(&line[..line.len() - 2]).into_owned()
            }
        };
        let Some((username, password)) = decode_plain(&response) else {
            self.config
                .metrics
                .authentication_failures
                .fetch_add(1, Ordering::Relaxed);
            if let Err(error) = self.record_auth_failure("invalid-authcid").await {
                self.log_rate_limit_error(&error);
                self.config
                    .metrics
                    .authentication_backend_failures
                    .fetch_add(1, Ordering::Relaxed);
                write_reply(
                    writer,
                    Reply::new(454, "authentication temporarily unavailable"),
                )
                .await?;
                return Ok(());
            }
            write_reply(writer, Reply::new(501, "invalid authentication response")).await?;
            return Ok(());
        };
        let account_limited = match self.is_auth_limited(&username).await {
            Ok(limited) => limited,
            Err(error) => {
                self.log_rate_limit_error(&error);
                self.config
                    .metrics
                    .authentication_backend_failures
                    .fetch_add(1, Ordering::Relaxed);
                write_reply(
                    writer,
                    Reply::new(454, "authentication temporarily unavailable"),
                )
                .await?;
                return Ok(());
            }
        };
        if account_limited {
            self.config
                .metrics
                .authentication_throttled
                .fetch_add(1, Ordering::Relaxed);
            write_reply(
                writer,
                Reply::new(454, "authentication temporarily unavailable"),
            )
            .await?;
            return Ok(());
        }
        match authenticator.authenticate(&username, &password).await {
            Ok(Some(identity)) => {
                if let Err(error) = self.record_auth_success(&username).await {
                    self.log_rate_limit_error(&error);
                    self.config
                        .metrics
                        .authentication_backend_failures
                        .fetch_add(1, Ordering::Relaxed);
                    write_reply(
                        writer,
                        Reply::new(454, "authentication temporarily unavailable"),
                    )
                    .await?;
                    return Ok(());
                }
                self.authenticated_identity = Some(identity);
                self.state = State::Authenticated;
                write_reply(writer, Reply::new(235, "authentication successful")).await?;
            }
            Ok(None) => {
                if let Err(error) = self.record_auth_failure(&username).await {
                    self.log_rate_limit_error(&error);
                    self.config
                        .metrics
                        .authentication_backend_failures
                        .fetch_add(1, Ordering::Relaxed);
                    write_reply(
                        writer,
                        Reply::new(454, "authentication temporarily unavailable"),
                    )
                    .await?;
                    return Ok(());
                }
                self.config
                    .metrics
                    .authentication_failures
                    .fetch_add(1, Ordering::Relaxed);
                write_reply(
                    writer,
                    Reply::new(535, "authentication credentials invalid"),
                )
                .await?;
            }
            Err(error) => {
                self.config
                    .metrics
                    .authentication_backend_failures
                    .fetch_add(1, Ordering::Relaxed);
                let category = match error {
                    crate::auth::AuthenticationError::Database(_) => "database",
                    crate::auth::AuthenticationError::Hashing => "password-hashing",
                    crate::auth::AuthenticationError::InvalidUsername => "invalid-username",
                    crate::auth::AuthenticationError::TenantUnavailable => "tenant-unavailable",
                    crate::auth::AuthenticationError::InvalidPassword => "invalid-password",
                    crate::auth::AuthenticationError::InvalidRateLimitKey => {
                        "invalid-rate-limit-key"
                    }
                };
                eprintln!("smtp authentication backend failure: {category}");
                write_reply(
                    writer,
                    Reply::new(454, "temporary authentication backend failure"),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn is_ip_limited(&self) -> Result<bool, AuthenticationError> {
        let local_limited = self.config.auth_rate_limiter.is_ip_limited(self.peer_ip);
        match &self.config.shared_auth_rate_limiter {
            Some(limiter) => Ok(local_limited || limiter.is_ip_limited(self.peer_ip).await?),
            None => Ok(local_limited),
        }
    }

    async fn is_auth_limited(&self, username: &str) -> Result<bool, AuthenticationError> {
        let local_limited = self
            .config
            .auth_rate_limiter
            .is_limited(self.peer_ip, username);
        match &self.config.shared_auth_rate_limiter {
            Some(limiter) => Ok(local_limited || limiter.is_limited(self.peer_ip, username).await?),
            None => Ok(local_limited),
        }
    }

    async fn record_auth_failure(&self, username: &str) -> Result<(), AuthenticationError> {
        self.config
            .auth_rate_limiter
            .record_failure(self.peer_ip, username);
        if let Some(limiter) = &self.config.shared_auth_rate_limiter {
            limiter.record_failure(self.peer_ip, username).await?;
        }
        Ok(())
    }

    async fn record_auth_success(&self, username: &str) -> Result<(), AuthenticationError> {
        self.config
            .auth_rate_limiter
            .record_success(self.peer_ip, username);
        if let Some(limiter) = &self.config.shared_auth_rate_limiter {
            limiter.record_success(self.peer_ip, username).await?;
        }
        Ok(())
    }

    fn log_rate_limit_error(&self, error: &AuthenticationError) {
        let category = match error {
            AuthenticationError::Database(_) => "database",
            _ => "configuration",
        };
        eprintln!("smtp authentication rate limiter failure: {category}");
    }

    async fn read_data<S>(&self, stream: &mut S) -> Result<Vec<u8>, SmtpError>
    where
        S: AsyncBufRead + Unpin,
    {
        let mut body = Vec::new();
        let mut line = Vec::with_capacity(self.config.max_line_length);
        let mut line_too_long = false;
        let mut oversized = false;
        let mut undeclared_8bit = false;
        let mut non_ascii_headers = false;
        let mut in_headers = true;
        loop {
            line.clear();
            let read = match timeout(
                self.config.command_timeout,
                read_bounded(
                    stream,
                    &mut line,
                    self.config.max_line_length,
                    self.config.max_buffered_input,
                ),
            )
            .await
            {
                Err(_) => return Err(SmtpError::Timeout),
                Ok(Err(SmtpError::LineTooLong)) => {
                    line_too_long = true;
                    continue;
                }
                Ok(Err(error)) => return Err(error),
                Ok(Ok(read)) => read,
            };
            if read == 0 {
                return Err(SmtpError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "disconnect during DATA",
                )));
            }
            if !line.ends_with(b"\r\n") {
                return Err(SmtpError::InvalidLineEnding);
            }
            if line[..line.len() - 2].contains(&b'\r') {
                return Err(SmtpError::InvalidLineEnding);
            }
            if line == b".\r\n" {
                return match (oversized, line_too_long, non_ascii_headers, undeclared_8bit) {
                    (true, _, _, _) => Err(SmtpError::MessageTooLarge),
                    (_, true, _, _) => Err(SmtpError::LineTooLong),
                    (false, false, true, _) => Err(SmtpError::InvalidHeaderEncoding),
                    (false, false, false, true) => Err(SmtpError::EightBitMimeRequired),
                    (false, false, false, false) => Ok(body),
                };
            }
            let content = if line.starts_with(b".") {
                &line[1..]
            } else {
                &line[..]
            };
            let has_non_ascii = content.iter().any(|byte| !byte.is_ascii());
            if in_headers && has_non_ascii {
                non_ascii_headers = true;
            } else if !in_headers && !self.transaction_body_8bit && has_non_ascii {
                undeclared_8bit = true;
            }
            if content == b"\r\n" {
                in_headers = false;
            }
            if body.len().saturating_add(content.len()) > self.transaction_size_limit {
                oversized = true;
            } else if !oversized {
                body.extend_from_slice(content);
            }
        }
    }

    fn reset_transaction(&mut self) {
        self.sender = None;
        self.recipients.clear();
        self.transaction_size_limit = self.config.max_message_bytes;
        self.declared_message_size = None;
        self.transaction_body_8bit = false;
        if self.state != State::Closed {
            self.state = if self.authenticated_identity.is_some() {
                State::Authenticated
            } else {
                State::Greeted
            };
        }
    }
}

async fn read_bounded<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    buffer: &mut Vec<u8>,
    max_line: usize,
    max_buffer: usize,
) -> Result<usize, SmtpError> {
    let mut count = 0;
    loop {
        let byte = match reader.read_u8().await {
            Ok(byte) => byte,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof && count == 0 => {
                return Ok(0);
            }
            Err(error) => return Err(error.into()),
        };
        count += 1;
        if count <= max_buffer && count <= max_line {
            buffer.push(byte);
        }
        if byte == b'\n' {
            return if count > max_buffer || count > max_line {
                Err(SmtpError::LineTooLong)
            } else {
                Ok(count)
            };
        }
    }
}

async fn write_reply<W: AsyncWrite + Unpin>(writer: &mut W, reply: Reply) -> Result<(), SmtpError> {
    writer.write_all(reply.to_string().as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

fn parse_error_reply(error: ParseError) -> Reply {
    match error {
        ParseError::Empty
        | ParseError::MissingArgument
        | ParseError::MalformedPath
        | ParseError::MalformedParameter
        | ParseError::DuplicateParameter => Reply::new(501, "syntax error in parameters"),

        ParseError::UnsupportedParameter => Reply::new(555, "unsupported parameter"),
        ParseError::InvalidCommand => Reply::new(500, "invalid command syntax"),
    }
}

fn decode_plain(value: &str) -> Option<(String, String)> {
    let bytes = decode_base64(value)?;
    let mut fields = bytes.split(|byte| *byte == 0);
    let authorization_identity = fields.next()?;
    let username = String::from_utf8(fields.next()?.to_vec()).ok()?;
    let password = String::from_utf8(fields.next()?.to_vec()).ok()?;
    if !authorization_identity.is_empty() || fields.next().is_some() {
        return None;
    }
    Some((username, password))
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    if value.is_empty() || value.len() > 1000 || !value.is_ascii() {
        return None;
    }
    STANDARD.decode(value).ok()
}

#[cfg(test)]
mod tests {
    use super::{Session, State, decode_plain};
    use crate::{command::Command, config::Config};
    use async_trait::async_trait;
    use base64::Engine as _;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    struct TestAuthenticator;

    #[async_trait]
    impl crate::Authenticator for TestAuthenticator {
        async fn authenticate(
            &self,
            username: &str,
            password: &str,
        ) -> Result<Option<crate::AuthenticatedIdentity>, crate::AuthenticationError> {
            Ok((username == "alice" && password == "secret").then(|| {
                crate::AuthenticatedIdentity {
                    tenant_id: uuid::Uuid::nil(),
                    username: "alice".into(),
                }
            }))
        }
    }

    #[test]
    fn enforces_sequence_and_preserves_noop_state() {
        let mut session = Session::new(Config::default());
        assert_eq!(
            session
                .process(Command::MailFrom {
                    sender: "a@example.test".into(),
                    size: None,
                    body_8bit: false,
                    has_body_parameter: false,
                    has_esmtp_parameters: false,
                })
                .code,
            503
        );
        let ehlo = session.process(Command::Ehlo("client".into()));
        assert_eq!(ehlo.code, 250);
        assert_eq!(ehlo.lines[1], "SIZE 10485760");
        assert_eq!(
            session
                .process(Command::MailFrom {
                    sender: "a@example.test".into(),
                    size: None,
                    body_8bit: false,
                    has_body_parameter: false,
                    has_esmtp_parameters: false,
                })
                .code,
            250
        );
        assert_eq!(session.process(Command::Noop).code, 250);
        assert_eq!(session.state(), State::MailFromAccepted);
        assert_eq!(session.process(Command::Rset).code, 250);
        assert_eq!(session.state(), State::Greeted);
    }

    #[test]
    fn accepts_null_reverse_path() {
        let mut session = Session::new(Config::default());
        assert_eq!(session.process(Command::Ehlo("client".into())).code, 250);
        assert_eq!(
            session
                .process(Command::MailFrom {
                    sender: String::new(),
                    size: None,
                    body_8bit: false,
                    has_body_parameter: false,
                    has_esmtp_parameters: false,
                })
                .code,
            250
        );
    }

    #[test]
    fn requires_authentication_before_mail() {
        let config = Config {
            require_authentication: true,
            authenticator: Some(std::sync::Arc::new(TestAuthenticator)),
            ..Config::default()
        };
        let mut session = Session::new(config);
        assert_eq!(session.process(Command::Ehlo("client".into())).code, 250);
        assert_eq!(
            session
                .process(Command::MailFrom {
                    sender: "a@example.test".into(),
                    size: None,
                    body_8bit: false,
                    has_body_parameter: false,
                    has_esmtp_parameters: false,
                })
                .code,
            530
        );
    }

    #[test]
    fn sasl_plain_rejects_proxy_identity_and_malformed_base64() {
        let proxy_auth =
            base64::engine::general_purpose::STANDARD.encode(b"other-user\0alice\0secret");
        assert!(decode_plain(&proxy_auth).is_none());
        assert!(decode_plain("AGFsaWNlAHNlY3JldA==ignored").is_none());
    }

    #[tokio::test]
    async fn collects_and_unstuffs_message_data() {
        let (client, server) = tokio::io::duplex(4096);
        let (server_reader, mut server_writer) = tokio::io::split(server);
        let task = tokio::spawn(async move {
            let mut reader = BufReader::new(server_reader);
            Session::new(Config::default())
                .run(&mut reader, &mut server_writer)
                .await
                .expect("session")
        });
        let (client_reader, mut client_writer) = tokio::io::split(client);
        let mut client_reader = BufReader::new(client_reader);
        let mut response = String::new();
        client_reader.read_line(&mut response).await.unwrap();
        client_writer.write_all(b"EHLO test\r\n").await.unwrap();
        response.clear();
        client_reader.read_line(&mut response).await.unwrap();
        response.clear();
        client_reader.read_line(&mut response).await.unwrap();
        client_writer
            .write_all(
                b"MAIL FROM:<sender@example.test>\r\nRCPT TO:<recipient@example.test>\r\nDATA\r\n",
            )
            .await
            .unwrap();
        for _ in 0..3 {
            response.clear();
            client_reader.read_line(&mut response).await.unwrap();
        }
        client_writer
            .write_all(b"Subject: test\r\n\r\n..secret\r\n.\r\n")
            .await
            .unwrap();
        response.clear();
        client_reader.read_line(&mut response).await.unwrap();
        client_writer.write_all(b"QUIT\r\n").await.unwrap();
        response.clear();
        client_reader.read_line(&mut response).await.unwrap();
        let result = task.await.unwrap();
        assert_eq!(result.messages[0].body, b"Subject: test\r\n\r\n.secret\r\n");
    }
}
