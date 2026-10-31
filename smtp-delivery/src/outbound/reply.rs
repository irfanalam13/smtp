use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use crate::outcome::DeliveryError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmtpReply {
    pub code: u16,
    pub lines: Vec<String>,
}

pub async fn read_reply<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<SmtpReply, DeliveryError> {
    let mut total: usize = 0;
    let mut lines = Vec::new();
    let mut buffer = Vec::new();
    let mut reply_code = None;
    loop {
        buffer.clear();
        let read = reader
            .read_until(b'\n', &mut buffer)
            .await
            .map_err(|_| DeliveryError::Protocol("reply read failed".into()))?;
        if read == 0 || read > max_bytes || total.saturating_add(read) > max_bytes {
            return Err(DeliveryError::Protocol(
                "reply exceeds configured limit".into(),
            ));
        }
        total += read;
        if !buffer.ends_with(b"\r\n") || buffer.len() < 5 {
            return Err(DeliveryError::Protocol("malformed SMTP reply".into()));
        }
        let code = std::str::from_utf8(&buffer[..3])
            .ok()
            .and_then(|text| text.parse::<u16>().ok())
            .ok_or_else(|| DeliveryError::Protocol("invalid SMTP code".into()))?;
        if reply_code.is_some_and(|expected| expected != code) {
            return Err(DeliveryError::Protocol(
                "inconsistent SMTP multiline reply code".into(),
            ));
        }
        reply_code = Some(code);
        let separator = buffer[3];
        if separator != b' ' && separator != b'-' {
            return Err(DeliveryError::Protocol(
                "invalid SMTP reply separator".into(),
            ));
        }
        let text = std::str::from_utf8(&buffer[4..buffer.len() - 2])
            .map_err(|_| DeliveryError::Protocol("non-UTF8 SMTP reply".into()))?;
        lines.push(text.chars().take(512).collect::<String>());
        if separator == b' ' {
            return Ok(SmtpReply { code, lines });
        }
        if lines.len() > 100 {
            return Err(DeliveryError::Protocol("too many SMTP reply lines".into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn parses_multiline_reply() {
        let mut reader = BufReader::new(&b"250-example\r\n250 SIZE 10\r\n"[..]);
        let reply = read_reply(&mut reader, 1024).await.unwrap();
        assert_eq!(reply.code, 250);
        assert_eq!(reply.lines.len(), 2);
    }

    #[tokio::test]
    async fn rejects_malformed_reply() {
        let mut reader = BufReader::new(&b"bad\r\n"[..]);
        assert!(read_reply(&mut reader, 1024).await.is_err());
    }

    #[tokio::test]
    async fn rejects_inconsistent_multiline_reply_codes() {
        let mut reader = BufReader::new(&b"250-first\r\n550 final\r\n"[..]);
        assert!(read_reply(&mut reader, 1024).await.is_err());
    }
}
