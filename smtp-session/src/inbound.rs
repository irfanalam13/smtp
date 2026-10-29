use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum InboundError {
    #[error("recipient is not authorized")]
    Rejected,
    #[error("temporary inbound storage failure")]
    Temporary,
    #[error("message exceeds inbound limits")]
    TooLarge,
    #[error("invalid recipient")]
    InvalidRecipient,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InboundReceipt {
    pub message_id: Uuid,
    pub duplicate: bool,
}

#[async_trait]
pub trait InboundAcceptor: Send + Sync {
    async fn accept(
        &self,
        envelope_sender: &str,
        recipients: &[String],
        raw_message: &[u8],
    ) -> Result<InboundReceipt, InboundError>;
}

#[derive(Clone)]
pub struct PostgresInboundStore {
    pool: PgPool,
    max_message_bytes: usize,
}

impl PostgresInboundStore {
    pub async fn connect(
        database_url: &str,
        max_message_bytes: usize,
    ) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await?;
        Ok(Self {
            pool,
            max_message_bytes,
        })
    }

    pub fn from_pool(pool: PgPool, max_message_bytes: usize) -> Self {
        Self {
            pool,
            max_message_bytes,
        }
    }
}

#[async_trait]
impl InboundAcceptor for PostgresInboundStore {
    async fn accept(
        &self,
        envelope_sender: &str,
        recipients: &[String],
        raw_message: &[u8],
    ) -> Result<InboundReceipt, InboundError> {
        if raw_message.len() > self.max_message_bytes {
            return Err(InboundError::TooLarge);
        }
        let normalized = recipients
            .iter()
            .map(|recipient| normalize_recipient(recipient))
            .collect::<Result<Vec<_>, _>>()?;
        if normalized.is_empty() || normalized.len() != recipients.len() {
            return Err(InboundError::InvalidRecipient);
        }
        let idempotency_key = message_fingerprint(envelope_sender, &normalized, raw_message);
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| InboundError::Temporary)?;
        let mut mailbox_ids = Vec::with_capacity(normalized.len());
        for recipient in &normalized {
            let (local_part, domain) = recipient
                .split_once('@')
                .ok_or(InboundError::InvalidRecipient)?;
            let row = sqlx::query(
                "SELECT m.id FROM mailboxes m JOIN domains d ON d.id = m.domain_id
                 WHERE lower(d.domain) = lower($1) AND d.status = 'verified'
                   AND m.local_part = $2 AND m.status = 'active'",
            )
            .bind(domain)
            .bind(local_part)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| InboundError::Temporary)?;
            let Some(row) = row else {
                return Err(InboundError::Rejected);
            };
            mailbox_ids.push(
                row.try_get::<Uuid, _>("id")
                    .map_err(|_| InboundError::Temporary)?,
            );
        }
        let mailbox_id = mailbox_ids[0];
        if mailbox_ids.iter().any(|id| *id != mailbox_id) {
            return Err(InboundError::Rejected);
        }
        if let Some(row) = sqlx::query(
            "SELECT id FROM inbound_messages WHERE mailbox_id = $1 AND idempotency_key = $2",
        )
        .bind(mailbox_id)
        .bind(idempotency_key.as_slice())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| InboundError::Temporary)?
        {
            let id = row.try_get("id").map_err(|_| InboundError::Temporary)?;
            transaction
                .commit()
                .await
                .map_err(|_| InboundError::Temporary)?;
            return Ok(InboundReceipt {
                message_id: id,
                duplicate: true,
            });
        }
        let message_id = Uuid::new_v4();
        let subject = extract_subject(raw_message);
        sqlx::query(
            "INSERT INTO inbound_messages
             (id, tenant_id, mailbox_id, envelope_sender, raw_message, raw_size, subject, idempotency_key)
             SELECT $1, tenant_id, $2, $3, $4, $5, $6, $7 FROM mailboxes WHERE id = $2",
        )
        .bind(message_id)
        .bind(mailbox_id)
        .bind(envelope_sender)
        .bind(raw_message)
        .bind(raw_message.len() as i64)
        .bind(subject)
        .bind(idempotency_key.as_slice())
        .execute(&mut *transaction)
        .await
        .map_err(|_| InboundError::Temporary)?;
        for recipient in normalized {
            sqlx::query("INSERT INTO inbound_recipients (message_id, recipient) VALUES ($1, $2)")
                .bind(message_id)
                .bind(recipient)
                .execute(&mut *transaction)
                .await
                .map_err(|_| InboundError::Temporary)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| InboundError::Temporary)?;
        Ok(InboundReceipt {
            message_id,
            duplicate: false,
        })
    }
}

fn message_fingerprint(sender: &str, recipients: &[String], body: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(sender.as_bytes());
    for recipient in recipients {
        hasher.update([0]);
        hasher.update(recipient.as_bytes());
    }
    hasher.update([0xff]);
    hasher.update(body);
    hasher.finalize().into()
}

pub fn normalize_recipient(raw: &str) -> Result<String, InboundError> {
    let (local, domain) = raw.rsplit_once('@').ok_or(InboundError::InvalidRecipient)?;
    let local = local.trim();
    let domain = domain.trim_end_matches('.').to_ascii_lowercase();
    if local.is_empty()
        || local.len() > 64
        || domain.is_empty()
        || domain.len() > 253
        || local.chars().any(|c| c.is_whitespace() || c.is_control())
        || domain.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(InboundError::InvalidRecipient);
    }
    Ok(format!("{local}@{domain}"))
}

fn extract_subject(raw: &[u8]) -> Option<String> {
    for line in raw.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix(b"Subject:") {
            return std::str::from_utf8(value)
                .ok()
                .map(|subject| subject.trim().chars().take(998).collect());
        }
    }
    None
}

#[allow(dead_code)]
fn received_at() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_only_domain_and_preserves_local_part() {
        assert_eq!(
            normalize_recipient("Alice@Example.COM.").unwrap(),
            "Alice@example.com"
        );
        assert!(normalize_recipient("alice+tag@example.com").is_ok());
        assert!(normalize_recipient("alice@example").is_ok());
        assert!(normalize_recipient("bad address@example.com").is_err());
    }

    #[test]
    fn extracts_bounded_untrusted_subject() {
        assert_eq!(
            extract_subject(b"Subject: hello\r\n\r\nbody"),
            Some("hello".into())
        );
        assert_eq!(extract_subject(b"X-Test: yes\r\n\r\nbody"), None);
    }
}
