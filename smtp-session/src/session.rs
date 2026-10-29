use std::io;

use tokio::{
    io::{AsyncBufRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::timeout,
};

use crate::inbound::InboundError;
use crate::{
    command::{self, Command, ParseError},
    config::Config,
    error::SmtpError,
    message::Message,
    reply::Reply,
};
use mail_queue::EnqueueRequest;
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

pub struct Session {
    config: Config,
    state: State,
    sender: Option<String>,
    recipients: Vec<String>,
    authenticated: bool,
    tls_active: bool,
}

impl Session {
    pub fn new(config: Config) -> Self {
        Self::new_with_transport(config, false)
    }

    pub fn new_with_transport(config: Config, tls_active: bool) -> Self {
        Self {
            config,
            state: State::Connected,
            sender: None,
            recipients: Vec::new(),
            authenticated: false,
            tls_active,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn process(&mut self, command: Command) -> Reply {
        match command {
            Command::Ehlo(_) => {
                self.reset_transaction();
                let mut capabilities = vec![
                    self.config.server_name.clone(),
                    format!("SIZE {}", self.config.max_message_bytes),
                ];
                if self.config.enable_8bitmime {
                    capabilities.push("8BITMIME".into());
                }
                if self.config.authenticator.is_some() {
                    capabilities.push("AUTH PLAIN".into());
                }
                Reply::multiline(250, capabilities)
            }
            Command::Helo(_) => {
                self.reset_transaction();
                Reply::new(250, self.config.server_name.clone())
            }
            Command::MailFrom {
                sender,
                size,
                body_8bit,
            } if matches!(self.state, State::Greeted | State::Authenticated)
                && (!self.config.require_authentication || self.authenticated) =>
            {
                if !self.config.enable_8bitmime && body_8bit {
                    return Reply::new(555, "8BITMIME parameter not supported");
                }
                if size.is_some_and(|size| size > self.config.max_message_bytes) {
                    return Reply::new(552, "message size exceeds fixed maximum");
                }
                self.sender = Some(sender);
                self.recipients.clear();
                self.state = State::MailFromAccepted;
                Reply::new(250, "sender ok")
            }
            Command::MailFrom { .. }
                if self.config.require_authentication && !self.authenticated =>
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
        reader: &mut R,
        writer: &mut W,
    ) -> Result<Vec<Message>, SmtpError>
    where
        R: AsyncBufRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        write_reply(writer, Reply::new(220, "smtp-test ready")).await?;
        let mut buffer = Vec::with_capacity(self.config.max_line_length.min(256));
        let mut messages = Vec::new();
        loop {
            buffer.clear();
            let read = match timeout(
                self.config.command_timeout,
                read_bounded(
                    reader,
                    &mut buffer,
                    self.config.max_line_length,
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
                return Ok(messages);
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
            let reply = self.process(command);
            write_reply(writer, reply).await?;
            if self.state == State::Closed {
                return Ok(messages);
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
                    Err(error) => return Err(error),
                };
                let message = Message {
                    sender: self.sender.clone().expect("DATA requires sender"),
                    recipients: self.recipients.clone(),
                    body,
                };
                self.config
                    .metrics
                    .message_bytes
                    .fetch_add(message.body.len() as u64, Ordering::Relaxed);
                if self.config.inbound.is_none()
                    && let Some(queue) = &self.config.queue
                {
                    let request = EnqueueRequest {
                        sender: message.sender.clone(),
                        recipients: message.recipients.clone(),
                        body: message.body.clone(),
                        idempotency_key: Uuid::new_v4().to_string(),
                        idempotency_scope: "smtp-loopback".to_string(),
                    };
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
                        .accept(&message.sender, &message.recipients, &message.body)
                        .await
                {
                    let temporary = matches!(&error, InboundError::Temporary);
                    let reply = match error {
                        InboundError::Rejected | InboundError::InvalidRecipient => {
                            Reply::new(550, "recipient not accepted")
                        }
                        InboundError::TooLarge => Reply::new(552, "message too large"),
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
                messages.push(message);
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
        let Some(authenticator) = self.config.authenticator.clone() else {
            write_reply(writer, Reply::new(502, "authentication not available")).await?;
            return Ok(());
        };
        if self.authenticated
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
                    self.config.max_line_length,
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
            write_reply(writer, Reply::new(501, "invalid authentication response")).await?;
            return Ok(());
        };
        if authenticator.authenticate(&username, &password).await {
            self.authenticated = true;
            self.state = State::Authenticated;
            write_reply(writer, Reply::new(235, "authentication successful")).await?;
        } else {
            write_reply(
                writer,
                Reply::new(535, "authentication credentials invalid"),
            )
            .await?;
        }
        Ok(())
    }

    async fn read_data<S>(&self, stream: &mut S) -> Result<Vec<u8>, SmtpError>
    where
        S: AsyncBufRead + Unpin,
    {
        let mut body = Vec::new();
        let mut line = Vec::with_capacity(self.config.max_line_length);
        loop {
            line.clear();
            let read = timeout(
                self.config.command_timeout,
                read_bounded(
                    stream,
                    &mut line,
                    self.config.max_line_length,
                    self.config.max_buffered_input,
                ),
            )
            .await
            .map_err(|_| SmtpError::Timeout)??;
            if read == 0 {
                return Err(SmtpError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "disconnect during DATA",
                )));
            }
            if !line.ends_with(b"\r\n") {
                return Err(SmtpError::InvalidLineEnding);
            }
            if line == b".\r\n" {
                return Ok(body);
            }
            let content = if line.starts_with(b"..") {
                &line[1..]
            } else {
                &line[..]
            };
            if body.len() + content.len() > self.config.max_message_bytes {
                return Err(SmtpError::MessageTooLarge);
            }
            body.extend_from_slice(content);
        }
    }

    fn reset_transaction(&mut self) {
        self.sender = None;
        self.recipients.clear();
        if self.state != State::Closed {
            self.state = if self.authenticated {
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
        if count <= max_buffer && count <= max_line + 2 {
            buffer.push(byte);
        }
        if byte == b'\n' {
            return if count > max_buffer || count > max_line + 2 {
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
        | ParseError::MalformedParameter => Reply::new(501, "syntax error in parameters"),

        ParseError::UnsupportedParameter => Reply::new(501, "unsupported parameter"),
    }
}

fn decode_plain(value: &str) -> Option<(String, String)> {
    let bytes = decode_base64(value)?;
    let mut fields = bytes.split(|byte| *byte == 0);
    let _authorization_identity = fields.next()?;
    let username = String::from_utf8(fields.next()?.to_vec()).ok()?;
    let password = String::from_utf8(fields.next()?.to_vec()).ok()?;
    if fields.next().is_some() {
        return None;
    }
    Some((username, password))
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    if value.is_empty() || !value.is_ascii() {
        return None;
    }
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for character in value.bytes() {
        if character == b'=' {
            break;
        }
        let digit = match character {
            b'A'..=b'Z' => character - b'A',
            b'a'..=b'z' => character - b'a' + 26,
            b'0'..=b'9' => character - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(digit);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    (bits < 6).then_some(output)
}

#[cfg(test)]
mod tests {
    use super::{Session, State};
    use crate::{command::Command, config::Config};
    use async_trait::async_trait;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    struct TestAuthenticator;

    #[async_trait]
    impl crate::Authenticator for TestAuthenticator {
        async fn authenticate(&self, username: &str, password: &str) -> bool {
            username == "alice" && password == "secret"
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
                })
                .code,
            250
        );
    }

    #[test]
    fn requires_authentication_before_mail() {
        let mut config = Config::default();
        config.require_authentication = true;
        config.authenticator = Some(std::sync::Arc::new(TestAuthenticator));
        let mut session = Session::new(config);
        assert_eq!(session.process(Command::Ehlo("client".into())).code, 250);
        assert_eq!(
            session
                .process(Command::MailFrom {
                    sender: "a@example.test".into(),
                    size: None,
                    body_8bit: false,
                })
                .code,
            530
        );
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
        client_writer.write_all(b"..secret\r\n.\r\n").await.unwrap();
        response.clear();
        client_reader.read_line(&mut response).await.unwrap();
        client_writer.write_all(b"QUIT\r\n").await.unwrap();
        response.clear();
        client_reader.read_line(&mut response).await.unwrap();
        let messages = task.await.unwrap();
        assert_eq!(messages[0].body, b".secret\r\n");
    }
}
