pub mod scanner;

use std::{
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    email_auth::{AuthenticationDisposition, AuthenticationError, EmailAuthenticator},
    scanner::{ClamdScanner, ScanError, SpamAssassinScanner},
};
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use shared_types::{CANONICAL_MESSAGE_SCHEMA_VERSION, CanonicalMessage};
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgPoolOptions};
use std::collections::HashMap;
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
    #[error("mailbox storage quota is exceeded")]
    QuotaExceeded,
    #[error("message was rejected by malware policy")]
    MalwareDetected,
    #[error("message was rejected by spam policy")]
    SpamDetected,
    #[error("message was rejected by sender authentication policy")]
    AuthenticationRejected,
    #[error("inbound traffic rate limit was exceeded")]
    RateLimited,
    #[error("invalid recipient")]
    InvalidRecipient,
    #[error("message metadata is invalid")]
    InvalidMessage,
    #[error("inbound rate-limit configuration is invalid")]
    InvalidRateLimitConfiguration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InboundReceipt {
    pub message_id: Uuid,
    pub duplicate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InboundContext {
    pub peer_ip: IpAddr,
    pub helo_domain: String,
    pub received_by: String,
    pub received_over_tls: bool,
}

#[async_trait]
pub trait InboundAcceptor: Send + Sync {
    async fn max_message_bytes(
        &self,
        _recipients: &[String],
    ) -> Result<Option<usize>, InboundError> {
        Ok(None)
    }

    async fn reserve_rate_capacity(
        &self,
        _peer_ip: IpAddr,
        _recipients: &[String],
    ) -> Result<(), InboundError> {
        Ok(())
    }

    async fn accept(
        &self,
        message: &CanonicalMessage,
        context: &InboundContext,
    ) -> Result<InboundReceipt, InboundError>;
}

#[derive(Clone)]
pub struct PostgresInboundStore {
    pool: PgPool,
    max_message_bytes: usize,
    scanner: Option<ClamdScanner>,
    spam_scanner: Option<SpamAssassinScanner>,
    email_authenticator: Option<EmailAuthenticator>,
    rate_limiter: Option<PostgresInboundRateLimiter>,
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
            scanner: None,
            spam_scanner: None,
            email_authenticator: None,
            rate_limiter: None,
        })
    }

    pub fn from_pool(pool: PgPool, max_message_bytes: usize) -> Self {
        Self {
            pool,
            max_message_bytes,
            scanner: None,
            spam_scanner: None,
            email_authenticator: None,
            rate_limiter: None,
        }
    }

    pub fn with_scanner(mut self, scanner: ClamdScanner) -> Self {
        self.scanner = Some(scanner);
        self
    }

    pub fn with_spam_scanner(mut self, scanner: SpamAssassinScanner) -> Self {
        self.spam_scanner = Some(scanner);
        self
    }

    pub fn with_email_authenticator(mut self, authenticator: EmailAuthenticator) -> Self {
        self.email_authenticator = Some(authenticator);
        self
    }

    pub fn with_rate_limiter(
        mut self,
        hmac_key: &[u8],
        messages_per_hour: u32,
    ) -> Result<Self, InboundError> {
        self.rate_limiter = Some(PostgresInboundRateLimiter::new(
            hmac_key,
            messages_per_hour,
        )?);
        Ok(self)
    }
}

#[derive(Clone)]
struct PostgresInboundRateLimiter {
    hmac_key: Arc<[u8]>,
    messages_per_hour: u32,
    cleanup_tick: Arc<AtomicU64>,
}

impl PostgresInboundRateLimiter {
    fn new(hmac_key: &[u8], messages_per_hour: u32) -> Result<Self, InboundError> {
        if hmac_key.len() < 32 || messages_per_hour == 0 {
            return Err(InboundError::InvalidRateLimitConfiguration);
        }
        Ok(Self {
            hmac_key: Arc::from(hmac_key),
            messages_per_hour,
            cleanup_tick: Arc::new(AtomicU64::new(0)),
        })
    }

    fn key_hash(&self, dimension: &str, value: &str) -> Result<[u8; 32], InboundError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.hmac_key)
            .map_err(|_| InboundError::InvalidMessage)?;
        mac.update(b"smtp-platform/inbound-rate/v1\0");
        mac.update(dimension.as_bytes());
        mac.update(&[0]);
        mac.update(value.as_bytes());
        Ok(mac.finalize().into_bytes().into())
    }

    async fn reserve(
        &self,
        pool: &PgPool,
        peer_ip: IpAddr,
        recipients: &[String],
    ) -> Result<(), InboundError> {
        if recipients.is_empty() {
            return Err(InboundError::InvalidRecipient);
        }
        let mut transaction = pool.begin().await.map_err(|_| InboundError::Temporary)?;
        self.reserve_key(&mut transaction, "ip", &peer_ip.to_string())
            .await?;
        if self
            .cleanup_tick
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(256)
        {
            sqlx::query(
                "DELETE FROM smtp_inbound_rate_limits
                 WHERE key_hash IN (
                     SELECT key_hash FROM smtp_inbound_rate_limits
                     WHERE updated_at < NOW() - INTERVAL '2 days'
                     ORDER BY updated_at LIMIT 1000
                 )",
            )
            .execute(&mut *transaction)
            .await
            .map_err(|_| InboundError::Temporary)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| InboundError::Temporary)
    }

    async fn reserve_domains(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        recipients: &[String],
    ) -> Result<(), InboundError> {
        let mut domains = recipients
            .iter()
            .map(|recipient| {
                let normalized = normalize_recipient(recipient)?;
                normalized
                    .rsplit_once('@')
                    .map(|(_, domain)| domain.to_owned())
                    .ok_or(InboundError::InvalidRecipient)
            })
            .collect::<Result<Vec<_>, _>>()?;
        domains.sort();
        domains.dedup();
        for domain in domains {
            self.reserve_key(transaction, "domain", &domain).await?;
        }
        Ok(())
    }

    async fn reserve_key(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        dimension: &str,
        value: &str,
    ) -> Result<(), InboundError> {
        let key = self.key_hash(dimension, value)?;
        let count: i32 = sqlx::query_scalar(
            "INSERT INTO smtp_inbound_rate_limits
                (key_hash, dimension, window_started_at, accepted_messages)
             VALUES ($1, $2, NOW(), 1)
             ON CONFLICT (key_hash) DO UPDATE
             SET accepted_messages = CASE
                     WHEN smtp_inbound_rate_limits.window_started_at <= NOW() - INTERVAL '1 hour'
                     THEN 1 ELSE smtp_inbound_rate_limits.accepted_messages + 1 END,
                 window_started_at = CASE
                     WHEN smtp_inbound_rate_limits.window_started_at <= NOW() - INTERVAL '1 hour'
                     THEN NOW() ELSE smtp_inbound_rate_limits.window_started_at END,
                 updated_at = NOW()
             RETURNING accepted_messages",
        )
        .bind(key.as_slice())
        .bind(dimension)
        .fetch_one(&mut **transaction)
        .await
        .map_err(|_| InboundError::Temporary)?;
        if count as u32 > self.messages_per_hour {
            return Err(InboundError::RateLimited);
        }
        Ok(())
    }
}

#[async_trait]
impl InboundAcceptor for PostgresInboundStore {
    async fn max_message_bytes(
        &self,
        recipients: &[String],
    ) -> Result<Option<usize>, InboundError> {
        let limit: Option<i64> = sqlx::query_scalar(
            "SELECT MIN(LEAST(
                 tenant.max_smtp_message_bytes,
                 COALESCE(domain.max_smtp_message_bytes, tenant.max_smtp_message_bytes)
             ))
             FROM unnest($1::text[]) AS recipient(address)
             JOIN domains AS domain
               ON lower(domain.domain) = lower(split_part(recipient.address, '@', 2))
              AND domain.status = 'verified'
             JOIN tenants AS tenant
               ON tenant.id = domain.tenant_id AND tenant.status = 'active'
             JOIN mailbox_addresses AS address
               ON address.domain_id = domain.id
              AND lower(address.local_part) = lower(split_part(recipient.address, '@', 1))
             LEFT JOIN mailbox_aliases AS alias
               ON alias.id = address.alias_id AND alias.status = 'active'
             JOIN mailboxes AS mailbox
               ON mailbox.id = address.mailbox_id
              AND mailbox.status = 'active'
             WHERE address.alias_id IS NULL OR alias.id IS NOT NULL",
        )
        .bind(recipients)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| InboundError::Temporary)?;
        limit
            .map(usize::try_from)
            .transpose()
            .map_err(|_| InboundError::Temporary)
    }

    async fn reserve_rate_capacity(
        &self,
        peer_ip: IpAddr,
        recipients: &[String],
    ) -> Result<(), InboundError> {
        if let Some(limiter) = &self.rate_limiter {
            limiter.reserve(&self.pool, peer_ip, recipients).await?;
        }
        Ok(())
    }

    async fn accept(
        &self,
        message: &CanonicalMessage,
        context: &InboundContext,
    ) -> Result<InboundReceipt, InboundError> {
        let envelope = message.envelope();
        let raw_message = message.raw_message();
        if message.schema_version() != CANONICAL_MESSAGE_SCHEMA_VERSION {
            return Err(InboundError::InvalidMessage);
        }
        if raw_message.len() > self.max_message_bytes {
            return Err(InboundError::TooLarge);
        }
        if context.received_by.is_empty()
            || context.received_by.len() > 255
            || !context
                .received_by
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b' ')
        {
            return Err(InboundError::InvalidMessage);
        }
        let (authentication_results, authentication_disposition) =
            if let Some(authenticator) = &self.email_authenticator {
                let (summary, disposition) = authenticator
                    .evaluate(
                        raw_message,
                        context.peer_ip,
                        &context.helo_domain,
                        &context.received_by,
                        &envelope.sender,
                    )
                    .await
                    .map_err(|error| match error {
                        AuthenticationError::InvalidMessage => InboundError::InvalidMessage,
                        AuthenticationError::Temporary => InboundError::Temporary,
                    })?;
                (Some(summary), disposition)
            } else {
                (None, AuthenticationDisposition::Accept)
            };
        if let Some(results) = authentication_results
            .as_ref()
            .filter(|result| !result.rua.is_empty())
        {
            let event_fingerprint =
                dmarc_report_fingerprint(context.peer_ip, &envelope.sender, raw_message);
            let envelope_from_domain = envelope
                .sender
                .rsplit_once('@')
                .map_or_else(String::new, |(_, domain)| domain.to_ascii_lowercase());
            let report_destinations = results
                .rua
                .iter()
                .map(|destination| destination.address.as_str())
                .collect::<Vec<_>>();
            let report_destination_max_bytes = results
                .rua
                .iter()
                .map(|destination| i64::try_from(destination.max_size).unwrap_or(i64::MAX))
                .collect::<Vec<_>>();
            let insert = sqlx::query(
                "INSERT INTO dmarc_report_events
                 (id, event_fingerprint, source_ip, policy_domain, header_from,
                  envelope_from_domain, spf_domain, spf_scope, spf_result, dmarc_result,
                  dmarc_spf_result, dmarc_dkim_result, disposition,
                  published_policy, subdomain_policy, spf_alignment, dkim_alignment,
                  dkim_results, report_destinations, report_destination_max_bytes)
                 VALUES ($1, $2, $3::inet, $4, $4, $5, $6, $7, $8, $9, $10,
                         $11, $12, $13, $14, $15, $16, $17, $18, $19)
                 ON CONFLICT (event_fingerprint) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(event_fingerprint.as_slice())
            .bind(context.peer_ip.to_string())
            .bind(&results.author_domain)
            .bind(&envelope_from_domain)
            .bind(&results.spf_domain)
            .bind(results.spf_scope)
            .bind(results.spf)
            .bind(results.dmarc)
            .bind(results.dmarc_spf)
            .bind(results.dmarc_dkim)
            .bind(if results.dmarc == "fail" {
                results.policy
            } else {
                "none"
            })
            .bind(results.published_policy)
            .bind(results.subdomain_policy)
            .bind(results.spf_alignment)
            .bind(results.dkim_alignment)
            .bind(serde_json::to_value(&results.dkim).map_err(|_| InboundError::Temporary)?)
            .bind(report_destinations)
            .bind(report_destination_max_bytes)
            .execute(&self.pool)
            .await;
            if insert.is_err() {
                return Err(InboundError::Temporary);
            }
        }
        if authentication_disposition == AuthenticationDisposition::Reject {
            return Err(InboundError::AuthenticationRejected);
        }
        if let Some(scanner) = &self.scanner {
            match scanner.scan(raw_message).await {
                Ok(()) => {}
                Err(ScanError::Infected) => return Err(InboundError::MalwareDetected),
                Err(ScanError::Spam) => return Err(InboundError::InvalidMessage),
                Err(ScanError::Unavailable) => return Err(InboundError::Temporary),
            }
        }
        if let Some(scanner) = &self.spam_scanner {
            match scanner.scan(raw_message).await {
                Ok(()) => {}
                Err(ScanError::Spam) => return Err(InboundError::SpamDetected),
                Err(ScanError::Unavailable) => return Err(InboundError::Temporary),
                Err(ScanError::Infected) => return Err(InboundError::InvalidMessage),
            }
        }
        let normalized = envelope
            .recipients
            .iter()
            .map(|recipient| normalize_recipient(recipient))
            .collect::<Result<Vec<_>, _>>()?;
        if normalized.is_empty() || normalized.len() != envelope.recipients.len() {
            return Err(InboundError::InvalidRecipient);
        }
        let idempotency_key = message_fingerprint(&envelope.sender, &normalized, raw_message);
        let metadata = serde_json::to_value(message.indexed_metadata())
            .map_err(|_| InboundError::InvalidMessage)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| InboundError::Temporary)?;
        // Resolve every envelope recipient from one database snapshot. A
        // transaction may fan out to multiple mailboxes belonging to one
        // tenant; cross-tenant distribution is deliberately disabled because
        // it would expose one tenant's message to another tenant's mailbox.
        let rows = sqlx::query(
            "SELECT recipient.address, mailbox.id AS mailbox_id, mailbox.tenant_id
             FROM unnest($1::text[]) WITH ORDINALITY AS recipient(address, ordinal)
             JOIN domains AS domain
               ON lower(domain.domain) = lower(split_part(recipient.address, '@', 2))
              AND domain.status = 'verified'
             JOIN tenants AS tenant
               ON tenant.id = domain.tenant_id AND tenant.status = 'active'
             JOIN mailbox_addresses AS address
               ON address.domain_id = domain.id
              AND lower(address.local_part) = lower(split_part(recipient.address, '@', 1))
             LEFT JOIN mailbox_aliases AS alias
               ON alias.id = address.alias_id AND alias.status = 'active'
             JOIN mailboxes AS mailbox
               ON mailbox.id = address.mailbox_id AND mailbox.status = 'active'
             WHERE (address.alias_id IS NULL OR alias.id IS NOT NULL)
             ORDER BY recipient.ordinal",
        )
        .bind(&normalized)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| InboundError::Temporary)?;
        if rows.len() != normalized.len() {
            return Err(InboundError::Rejected);
        }
        let tenant_id: Uuid = rows[0]
            .try_get("tenant_id")
            .map_err(|_| InboundError::Temporary)?;
        if rows.iter().any(|row| {
            row.try_get::<Uuid, _>("tenant_id")
                .map_or(true, |resolved_tenant| resolved_tenant != tenant_id)
        }) {
            return Err(InboundError::Rejected);
        }

        let mut fanout: Vec<(Uuid, Vec<String>)> = Vec::new();
        let mut mailbox_indexes = HashMap::new();
        for row in rows {
            let mailbox_id: Uuid = row
                .try_get("mailbox_id")
                .map_err(|_| InboundError::Temporary)?;
            let recipient: String = row
                .try_get("address")
                .map_err(|_| InboundError::Temporary)?;
            let index = *mailbox_indexes.entry(mailbox_id).or_insert_with(|| {
                fanout.push((mailbox_id, Vec::new()));
                fanout.len() - 1
            });
            if !fanout[index].1.contains(&recipient) {
                fanout[index].1.push(recipient);
            }
        }

        // The fan-out and all recipient links commit together. An SMTP 250 is
        // sent only after this transaction commits, so no mailbox receives a
        // partial copy when another recipient cannot be persisted.
        let mut existing_ids = Vec::with_capacity(fanout.len());
        for (mailbox_id, mailbox_recipients) in &fanout {
            let existing = sqlx::query(
                "SELECT message.id, message.envelope_sender, message.raw_message,
                        ARRAY(SELECT recipient FROM inbound_recipients
                              WHERE message_id = message.id ORDER BY recipient) AS recipients
                 FROM inbound_messages AS message
                 WHERE message.mailbox_id = $1 AND message.idempotency_key = $2",
            )
            .bind(mailbox_id)
            .bind(idempotency_key.as_slice())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| InboundError::Temporary)?;
            if let Some(row) = existing {
                let stored_sender: String = row
                    .try_get("envelope_sender")
                    .map_err(|_| InboundError::Temporary)?;
                let stored_raw: Vec<u8> = row
                    .try_get("raw_message")
                    .map_err(|_| InboundError::Temporary)?;
                let mut stored_recipients: Vec<String> = row
                    .try_get("recipients")
                    .map_err(|_| InboundError::Temporary)?;
                let mut expected_recipients = mailbox_recipients.clone();
                stored_recipients.sort();
                expected_recipients.sort();
                // Never treat a digest match alone as an idempotent retry.
                if stored_sender != envelope.sender
                    || stored_raw != raw_message
                    || stored_recipients != expected_recipients
                {
                    return Err(InboundError::Temporary);
                }
                existing_ids.push(
                    row.try_get::<Uuid, _>("id")
                        .map_err(|_| InboundError::Temporary)?,
                );
            }
        }
        if existing_ids.len() == fanout.len() {
            transaction
                .commit()
                .await
                .map_err(|_| InboundError::Temporary)?;
            return Ok(InboundReceipt {
                message_id: existing_ids[0],
                duplicate: true,
            });
        }
        if !existing_ids.is_empty() {
            // A partially present fan-out indicates legacy/inconsistent state;
            // fail closed rather than silently reporting incomplete delivery.
            return Err(InboundError::Temporary);
        }
        if let Some(limiter) = &self.rate_limiter {
            limiter
                .reserve_domains(&mut transaction, &normalized)
                .await?;
        }
        let subject = message
            .parsed()
            .subject
            .as_deref()
            .map(|value| value.chars().take(998).collect::<String>());
        let mut message_id = None;
        for (mailbox_id, recipients) in fanout {
            let current_id = Uuid::new_v4();
            let quota = sqlx::query(
                "UPDATE mailboxes
                 SET used_bytes = used_bytes + $1
                 WHERE id = $2 AND status = 'active'
                   AND used_bytes + $1 <= quota_bytes",
            )
            .bind(i64::try_from(raw_message.len()).map_err(|_| InboundError::TooLarge)?)
            .bind(mailbox_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| InboundError::Temporary)?;
            if quota.rows_affected() != 1 {
                return Err(InboundError::QuotaExceeded);
            }
            sqlx::query(
                "INSERT INTO inbound_messages
                 (id, tenant_id, mailbox_id, envelope_sender, raw_message, raw_size, subject,
                  message_schema_version, parsed_metadata, idempotency_key, received_from_ip,
                  received_by, received_over_tls, authentication_results, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::inet, $12, $13, $14, $15)",
            )
            .bind(current_id)
            .bind(tenant_id)
            .bind(mailbox_id)
            .bind(&envelope.sender)
            .bind(raw_message)
            .bind(raw_message.len() as i64)
            .bind(subject.as_deref())
            .bind(
                i16::try_from(message.schema_version())
                    .map_err(|_| InboundError::InvalidMessage)?,
            )
            .bind(&metadata)
            .bind(idempotency_key.as_slice())
            .bind(context.peer_ip.to_string())
            .bind(&context.received_by)
            .bind(context.received_over_tls)
            .bind(
                authentication_results
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|_| InboundError::InvalidMessage)?,
            )
            .bind(
                if authentication_disposition == AuthenticationDisposition::Quarantine {
                    "quarantined"
                } else {
                    "stored"
                },
            )
            .execute(&mut *transaction)
            .await
            .map_err(|_| InboundError::Temporary)?;
            for recipient in recipients {
                sqlx::query(
                    "INSERT INTO inbound_recipients (message_id, recipient) VALUES ($1, $2)",
                )
                .bind(current_id)
                .bind(recipient)
                .execute(&mut *transaction)
                .await
                .map_err(|_| InboundError::Temporary)?;
            }
            sqlx::query(
                "INSERT INTO mailbox_event_outbox
                 (tenant_id, mailbox_id, event_type, idempotency_key, payload)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (tenant_id, event_type, idempotency_key) DO NOTHING",
            )
            .bind(tenant_id)
            .bind(mailbox_id)
            .bind(
                if authentication_disposition == AuthenticationDisposition::Quarantine {
                    "inbound.message.quarantined"
                } else {
                    "inbound.message.accepted"
                },
            )
            .bind(current_id.to_string())
            .bind(serde_json::json!({
                "mailbox_id": mailbox_id,
                "message_id": current_id,
                "authentication": authentication_results,
            }))
            .execute(&mut *transaction)
            .await
            .map_err(|_| InboundError::Temporary)?;
            message_id.get_or_insert(current_id);
        }
        transaction
            .commit()
            .await
            .map_err(|_| InboundError::Temporary)?;
        Ok(InboundReceipt {
            message_id: message_id.expect("non-empty recipient fan-out was validated"),
            duplicate: false,
        })
    }
}

fn message_fingerprint(sender: &str, recipients: &[String], body: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"smtp-platform/inbound-idempotency/v1\0");
    hasher.update((sender.len() as u64).to_be_bytes());
    hasher.update(sender.as_bytes());
    hasher.update((recipients.len() as u64).to_be_bytes());
    for recipient in recipients {
        hasher.update((recipient.len() as u64).to_be_bytes());
        hasher.update(recipient.as_bytes());
    }
    hasher.update((body.len() as u64).to_be_bytes());
    hasher.update(body);
    hasher.finalize().into()
}

fn dmarc_report_fingerprint(peer_ip: std::net::IpAddr, sender: &str, body: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"smtp-platform/dmarc-report-event/v1\0");
    hasher.update(peer_ip.to_string().as_bytes());
    hasher.update((sender.len() as u64).to_be_bytes());
    hasher.update(sender.as_bytes());
    hasher.update((body.len() as u64).to_be_bytes());
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
    fn fingerprint_frames_envelope_fields_and_message_bytes() {
        let first = message_fingerprint("a", &["bc".into()], b"body");
        let different_sender = message_fingerprint("ab", &["c".into()], b"body");
        let different_body = message_fingerprint("a", &["bc".into()], b"body\0");
        assert_ne!(first, different_sender);
        assert_ne!(first, different_body);
        assert_eq!(first, message_fingerprint("a", &["bc".into()], b"body"));
    }

    #[test]
    fn inbound_rate_limit_keys_are_validated_and_scope_separated() {
        assert!(matches!(
            PostgresInboundRateLimiter::new(b"short", 10),
            Err(InboundError::InvalidRateLimitConfiguration)
        ));
        assert!(matches!(
            PostgresInboundRateLimiter::new(&[7_u8; 32], 0),
            Err(InboundError::InvalidRateLimitConfiguration)
        ));
        let limiter = PostgresInboundRateLimiter::new(&[7_u8; 32], 10).unwrap();
        assert_ne!(
            limiter.key_hash("ip", "192.0.2.1").unwrap(),
            limiter.key_hash("domain", "192.0.2.1").unwrap()
        );
    }
}
