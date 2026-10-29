use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MailboxFlag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Recent,
}

impl MailboxFlag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Seen => "\\Seen",
            Self::Answered => "\\Answered",
            Self::Flagged => "\\Flagged",
            Self::Deleted => "\\Deleted",
            Self::Draft => "\\Draft",
            Self::Recent => "\\Recent",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxMessage {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub uid: u64,
    pub raw_message: Vec<u8>,
    pub flags: Vec<MailboxFlag>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MailboxError {
    #[error("mailbox name is empty")]
    EmptyName,
    #[error("mailbox name exceeds 255 bytes")]
    NameTooLong,
    #[error("mailbox name contains control characters")]
    ControlCharacter,
    #[error("mailbox local part must be 1 to 64 bytes and contain no @ or control characters")]
    InvalidLocalPart,
    #[error("message exceeds mailbox size limit")]
    MessageTooLarge,
}

pub fn validate_mailbox_name(name: &str) -> Result<(), MailboxError> {
    if name.is_empty() {
        return Err(MailboxError::EmptyName);
    }
    if name.len() > 255 {
        return Err(MailboxError::NameTooLong);
    }
    if name.chars().any(|character| character.is_control()) {
        return Err(MailboxError::ControlCharacter);
    }
    Ok(())
}

pub fn validate_local_part(local_part: &str) -> Result<(), MailboxError> {
    if local_part.is_empty()
        || local_part.len() > 64
        || local_part.contains('@')
        || local_part.trim() != local_part
        || local_part.chars().any(char::is_control)
    {
        return Err(MailboxError::InvalidLocalPart);
    }
    Ok(())
}

pub fn validate_message_size(raw_message: &[u8], max_bytes: usize) -> Result<(), MailboxError> {
    if raw_message.len() > max_bytes {
        return Err(MailboxError::MessageTooLarge);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MailboxError, validate_local_part, validate_mailbox_name, validate_message_size};

    #[test]
    fn validates_bounded_mailbox_names() {
        assert!(validate_mailbox_name("INBOX").is_ok());
        assert_eq!(validate_mailbox_name(""), Err(MailboxError::EmptyName));
        assert_eq!(
            validate_mailbox_name("bad\r\nname"),
            Err(MailboxError::ControlCharacter)
        );
    }

    #[test]
    fn enforces_raw_message_limit() {
        assert!(validate_message_size(b"body", 4).is_ok());
        assert_eq!(
            validate_message_size(b"body!", 4),
            Err(MailboxError::MessageTooLarge)
        );
    }

    #[test]
    fn validates_bounded_mailbox_local_parts() {
        assert!(validate_local_part("team").is_ok());
        assert_eq!(validate_local_part(""), Err(MailboxError::InvalidLocalPart));
        assert_eq!(
            validate_local_part("team@example.test"),
            Err(MailboxError::InvalidLocalPart)
        );
        assert_eq!(
            validate_local_part("team\r\n"),
            Err(MailboxError::InvalidLocalPart)
        );
    }
}
