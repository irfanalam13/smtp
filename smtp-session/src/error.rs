use std::{fmt, io};

#[derive(Debug)]
pub enum SmtpError {
    Io(io::Error),
    Timeout,
    LineTooLong,
    InvalidLineEnding,
    MessageTooLarge,
    ConnectionLimit,
    Persistence(String),
    Tls(String),
}

impl fmt::Display for SmtpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Timeout => f.write_str("client timeout"),
            Self::LineTooLong => f.write_str("line exceeds configured limit"),
            Self::InvalidLineEnding => f.write_str("line does not use CRLF"),
            Self::MessageTooLarge => f.write_str("message exceeds configured limit"),
            Self::ConnectionLimit => f.write_str("connection limit reached"),
            Self::Persistence(error) => write!(f, "persistence error: {error}"),
            Self::Tls(error) => write!(f, "TLS error: {error}"),
        }
    }
}

impl std::error::Error for SmtpError {}

impl From<mail_queue::QueueError> for SmtpError {
    fn from(error: mail_queue::QueueError) -> Self {
        Self::Persistence(error.to_string())
    }
}

impl From<io::Error> for SmtpError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
