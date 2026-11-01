use std::sync::Arc;

use mail_queue::{
    QueueError,
    model::{ClaimedDeliveryBatch, ClaimedJob, RecipientDeliveryResult, RecipientDeliveryState},
};
use shared_types::{
    CANONICAL_MESSAGE_SCHEMA_VERSION, CanonicalMessage, MessageEnvelope, MimeLimits,
};

use crate::{
    client::{classify_delivery_error, deliver, recipient_failure},
    config::DeliveryConfig,
    dkim::{SigningError, sign_message},
    dns::{Resolver, recipient_domain},
    outcome::DeliveryError,
};

pub struct SmtpProcessor<R = crate::dns::DnsResolver> {
    pub config: DeliveryConfig,
    pub resolver: Arc<R>,
}

impl<R: Resolver> SmtpProcessor<R> {
    pub async fn process(
        &self,
        batch: &ClaimedDeliveryBatch,
    ) -> Result<Vec<RecipientDeliveryResult>, QueueError> {
        if !self.config.enabled {
            return Err(QueueError::Processor("real delivery is disabled".into()));
        }
        let expected_mode = if let Some(relay) = &self.config.smtp_relay {
            match relay.mode {
                crate::config::SmtpRelayMode::AuthenticatedTls => "relay",
                crate::config::SmtpRelayMode::LocalMailpit => "mailpit",
            }
        } else {
            "direct_mx"
        };
        let dkim_reference_matches = match batch.sender_domain.as_deref() {
            Some(domain) => {
                batch.dkim_key_ref.as_deref() == Some(&format!("secret-manager://dkim/{domain}"))
            }
            None => false,
        };
        let sender_domain_matches = batch.sender_domain.as_deref()
            == batch
                .envelope_sender
                .rsplit_once('@')
                .map(|(_, domain)| domain.to_ascii_lowercase())
                .as_deref();
        let invalid_reason = if batch.delivery_mode != expected_mode {
            Some("queued delivery mode does not match this worker configuration".to_owned())
        } else if batch.delivery_mode == "direct_mx"
            && (batch.tenant_id.is_none() || !sender_domain_matches || !dkim_reference_matches)
        {
            Some("tenant sender or DKIM key reference is invalid".to_owned())
        } else if batch.schema_version != CANONICAL_MESSAGE_SCHEMA_VERSION {
            Some("unsupported canonical message schema version".to_owned())
        } else if batch.attempt_recipients.iter().any(|recipient| {
            !batch.recipients.contains(&recipient.address)
                || (self.config.smtp_relay.is_none()
                    && !matches!(
                    recipient_domain(&recipient.address),
                    Ok(domain)
                        if domain.trim_end_matches('.').eq_ignore_ascii_case(
                            batch.destination_domain.trim_end_matches('.')
                        )
                    ))
        }) {
            Some("claimed recipient does not match canonical envelope batch".to_owned())
        } else {
            match CanonicalMessage::parse(
                MessageEnvelope {
                    sender: batch.envelope_sender.clone(),
                    recipients: batch.recipients.clone(),
                },
                batch.raw_message.clone(),
                MimeLimits {
                    max_message_bytes: self.config.max_message_bytes,
                    ..MimeLimits::default()
                },
            ) {
                Ok(_) => None,
                Err(error) => Some(format!("canonical message validation failed: {error}")),
            }
        };
        if let Some(reason) = invalid_reason {
            return Ok(batch
                .attempt_recipients
                .iter()
                .map(|recipient| RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state: RecipientDeliveryState::PermanentFailure,
                    smtp_code: None,
                    diagnostic: Some(reason.clone()),
                    remote_host: None,
                })
                .collect());
        }

        let body = if let Some(key_dir) = &self.config.dkim_key_dir {
            match sign_message(key_dir, &batch.raw_message).await {
                Ok(signed) => signed,
                Err(error) => {
                    let state = match error {
                        SigningError::Unavailable => RecipientDeliveryState::TemporaryFailure,
                        SigningError::InvalidFrom | SigningError::InvalidKey => {
                            RecipientDeliveryState::PermanentFailure
                        }
                    };
                    return Ok(batch
                        .attempt_recipients
                        .iter()
                        .map(|recipient| RecipientDeliveryResult {
                            delivery_id: recipient.delivery_id,
                            state,
                            smtp_code: None,
                            diagnostic: Some(match state {
                                RecipientDeliveryState::TemporaryFailure => {
                                    "DKIM signing key is temporarily unavailable".into()
                                }
                                _ => "message could not be DKIM signed".into(),
                            }),
                            remote_host: None,
                        })
                        .collect());
                }
            }
        } else {
            batch.raw_message.clone()
        };
        let job = ClaimedJob {
            message_id: batch.message_id,
            sender: batch.envelope_sender.clone(),
            recipients: batch
                .attempt_recipients
                .iter()
                .map(|recipient| recipient.address.clone())
                .collect(),
            body,
            attempt_count: 1,
            lease_token: batch.lease_token,
            lease_expires_at: batch.lease_expires_at,
        };
        let reports = match deliver(&job, &self.config, self.resolver.as_ref()).await {
            Ok(report) => report.recipients,
            Err(DeliveryError::Disabled) => {
                return Err(QueueError::Processor("real delivery is disabled".into()));
            }
            Err(error) => {
                let status = classify_delivery_error(&error);
                job.recipients
                    .into_iter()
                    .map(|recipient| recipient_failure(recipient, status.clone(), &error, None))
                    .collect()
            }
        };

        let mut remaining = reports;
        batch
            .attempt_recipients
            .iter()
            .map(|recipient| {
                let position = remaining
                    .iter()
                    .position(|report| report.recipient == recipient.address)
                    .ok_or_else(|| {
                        QueueError::Processor(
                            "SMTP delivery omitted a claimed recipient result".into(),
                        )
                    })?;
                let report = remaining.remove(position);
                let state = match report.status {
                    crate::client::RecipientDeliveryStatus::AcceptedByRemote => {
                        RecipientDeliveryState::AcceptedByRemote
                    }
                    crate::client::RecipientDeliveryStatus::TemporaryFailure => {
                        RecipientDeliveryState::TemporaryFailure
                    }
                    crate::client::RecipientDeliveryStatus::PermanentFailure => {
                        RecipientDeliveryState::PermanentFailure
                    }
                    crate::client::RecipientDeliveryStatus::Uncertain => {
                        RecipientDeliveryState::Uncertain
                    }
                };
                Ok(RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state,
                    smtp_code: report.smtp_code,
                    diagnostic: Some(report.diagnostic),
                    remote_host: report.remote_mx,
                })
            })
            .collect::<Result<Vec<_>, QueueError>>()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;
    use mail_queue::model::{ClaimedDeliveryBatch, ClaimedRecipient};
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };
    use uuid::Uuid;

    use crate::{DeliveryConfig, DnsResolver};

    use super::*;

    #[tokio::test]
    async fn unsupported_message_schema_is_persisted_as_permanent_failure() {
        let processor = SmtpProcessor {
            config: DeliveryConfig {
                enabled: true,
                allowed_recipient_domains: vec!["example.test".into()],
                ..DeliveryConfig::default()
            },
            resolver: Arc::new(DnsResolver::default()),
        };
        let batch = ClaimedDeliveryBatch {
            message_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            envelope_sender: "sender@example.test".into(),
            sender_domain: Some("example.test".into()),
            delivery_mode: "direct_mx".into(),
            dkim_key_ref: Some("secret-manager://dkim/example.test".into()),
            recipients: vec!["recipient@example.test".into()],
            destination_domain: "example.test".into(),
            raw_message: b"Subject: test\r\n\r\nbody".to_vec(),
            schema_version: CANONICAL_MESSAGE_SCHEMA_VERSION + 1,
            created_at: Utc::now(),
            attempt_recipients: vec![ClaimedRecipient {
                delivery_id: 1,
                address: "recipient@example.test".into(),
                attempt_number: 1,
            }],
            lease_token: Uuid::new_v4(),
            lease_expires_at: Utc::now() + chrono::Duration::minutes(1),
        };

        let results = processor.process(&batch).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].state, RecipientDeliveryState::PermanentFailure);
        assert!(
            results[0]
                .diagnostic
                .as_deref()
                .unwrap()
                .contains("unsupported canonical message schema version")
        );
    }

    #[tokio::test]
    async fn canonical_stored_message_is_validated_then_sent_unchanged() {
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
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "Subject: canonical\r\n");
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "\r\n");
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "body\r\n");
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, ".\r\n");
            write_half.write_all(b"250 accepted\r\n").await.unwrap();
        });
        let processor = SmtpProcessor {
            config: DeliveryConfig {
                enabled: true,
                test_endpoint: Some(endpoint),
                allow_loopback_test_destinations: true,
                allowed_recipient_domains: vec!["example.test".into()],
                command_timeout: Duration::from_secs(2),
                overall_timeout: Duration::from_secs(5),
                ..DeliveryConfig::default()
            },
            resolver: Arc::new(DnsResolver::default()),
        };
        let batch = ClaimedDeliveryBatch {
            message_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            envelope_sender: "sender@example.test".into(),
            sender_domain: Some("example.test".into()),
            delivery_mode: "direct_mx".into(),
            dkim_key_ref: Some("secret-manager://dkim/example.test".into()),
            recipients: vec!["recipient@example.test".into()],
            destination_domain: "example.test".into(),
            raw_message: b"Subject: canonical\r\n\r\nbody".to_vec(),
            schema_version: CANONICAL_MESSAGE_SCHEMA_VERSION,
            created_at: Utc::now(),
            attempt_recipients: vec![ClaimedRecipient {
                delivery_id: 7,
                address: "recipient@example.test".into(),
                attempt_number: 1,
            }],
            lease_token: Uuid::new_v4(),
            lease_expires_at: Utc::now() + chrono::Duration::minutes(1),
        };

        let results = processor.process(&batch).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].state, RecipientDeliveryState::AcceptedByRemote);
        server.await.unwrap();
    }
}
