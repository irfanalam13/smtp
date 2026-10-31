use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::outcome::DeliveryError;

pub async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    body: &[u8],
    max_bytes: usize,
) -> Result<(), DeliveryError> {
    if body.len() > max_bytes {
        return Err(DeliveryError::MessageTooLarge);
    }
    let normalized = normalize_crlf(body)?;
    for line in normalized.split_inclusive(|byte| *byte == b'\n') {
        if line.starts_with(b".") {
            writer
                .write_all(b".")
                .await
                .map_err(|_| DeliveryError::AmbiguousFinalResponse)?;
        }
        writer
            .write_all(line)
            .await
            .map_err(|_| DeliveryError::AmbiguousFinalResponse)?;
    }
    if !normalized.ends_with(b"\r\n") {
        writer
            .write_all(b"\r\n")
            .await
            .map_err(|_| DeliveryError::AmbiguousFinalResponse)?;
    }
    writer
        .write_all(b".\r\n")
        .await
        .map_err(|_| DeliveryError::AmbiguousFinalResponse)?;
    writer
        .flush()
        .await
        .map_err(|_| DeliveryError::AmbiguousFinalResponse)?;
    Ok(())
}

fn normalize_crlf(body: &[u8]) -> Result<Vec<u8>, DeliveryError> {
    if body.windows(1).any(|window| window == [0]) {
        return Err(DeliveryError::Protocol("NUL in message".into()));
    }
    let mut result = Vec::with_capacity(body.len() + 2);
    let mut index = 0;
    while index < body.len() {
        if body[index] == b'\r' {
            if body.get(index + 1) != Some(&b'\n') {
                return Err(DeliveryError::Protocol("bare CR in message".into()));
            }
            result.extend_from_slice(b"\r\n");
            index += 2;
        } else if body[index] == b'\n' {
            result.extend_from_slice(b"\r\n");
            index += 1;
        } else {
            result.push(body[index]);
            index += 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dot_stuffs_and_frames_message() {
        let mut output = Vec::new();
        write_message(&mut output, b".one\nsecond", 100)
            .await
            .unwrap();
        assert_eq!(output, b"..one\r\nsecond\r\n.\r\n");
    }

    #[test]
    fn rejects_bare_cr() {
        assert!(normalize_crlf(b"bad\rcarriage").is_err());
    }
}
