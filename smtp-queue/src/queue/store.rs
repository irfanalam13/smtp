use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::{
    config::QueueConfig,
    error::QueueError,
    model::{
        ClaimedDeliveryBatch, ClaimedJob, ClaimedRecipient, EnqueueRequest, EnqueueResult,
        FailureCategory, MessageId, QueueState, RecipientDeliveryResult, RecipientDeliveryState,
    },
    retry,
};

#[derive(Clone)]
pub struct Queue {
    pool: PgPool,
    config: QueueConfig,
}

impl Queue {
    pub async fn connect(config: QueueConfig) -> Result<Self, QueueError> {
        config.validate()?;
        let pool = PgPoolOptions::new()
            .max_connections(config.max_pool_connections)
            .acquire_timeout(config.acquire_timeout)
            .connect(&config.database_url)
            .await?;
        Ok(Self { pool, config })
    }

    pub fn from_pool(pool: PgPool, config: QueueConfig) -> Result<Self, QueueError> {
        config.validate()?;
        Ok(Self { pool, config })
    }

    pub fn config(&self) -> &QueueConfig {
        &self.config
    }

    pub async fn enqueue(&self, request: &EnqueueRequest<'_>) -> Result<EnqueueResult, QueueError> {
        request.validate(&self.config)?;
        let fingerprint = fingerprint(request);
        let metadata = serde_json::to_value(request.canonical_message.indexed_metadata()).map_err(
            |error| QueueError::InvalidInput(format!("encode canonical message metadata: {error}")),
        )?;
        let mut transaction = self.pool.begin().await?;
        if let Some(row) = sqlx::query("SELECT message_id, fingerprint FROM mail_idempotency WHERE scope = $1 AND idempotency_key = $2 FOR UPDATE")
            .bind(&request.idempotency_scope)
            .bind(&request.idempotency_key)
            .fetch_optional(&mut *transaction)
            .await?
        {
            let existing: Vec<u8> = row.try_get("fingerprint")?;
            if existing != fingerprint {
                return Err(QueueError::IdempotencyConflict);
            }
            let message_id: MessageId = row.try_get("message_id")?;
            transaction.commit().await?;
            return Ok(EnqueueResult { message_id, created: false });
        }

        let message_id = Uuid::new_v4();
        sqlx::query("INSERT INTO mail_messages (id, envelope_sender, body, body_size, message_schema_version, parsed_metadata, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW())")
            .bind(message_id)
            .bind(&request.canonical_message.envelope().sender)
            .bind(request.canonical_message.raw_message())
            .bind(request.canonical_message.raw_message().len() as i64)
            .bind(i16::try_from(request.canonical_message.schema_version()).map_err(|_| {
                QueueError::InvalidInput("canonical message schema version is invalid".into())
            })?)
            .bind(metadata)
            .execute(&mut *transaction)
            .await?;
        for recipient in &request.canonical_message.envelope().recipients {
            sqlx::query("INSERT INTO mail_recipients (message_id, recipient) VALUES ($1, $2)")
                .bind(message_id)
                .bind(recipient)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query("INSERT INTO mail_queue (message_id, state, next_attempt_at, attempt_count, created_at, updated_at) VALUES ($1, 'queued', NOW(), 0, NOW(), NOW())")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        let inserted = sqlx::query("INSERT INTO mail_idempotency (scope, idempotency_key, fingerprint, message_id, created_at) VALUES ($1, $2, $3, $4, NOW()) ON CONFLICT (scope, idempotency_key) DO NOTHING")
            .bind(&request.idempotency_scope)
            .bind(&request.idempotency_key)
            .bind(&fingerprint)
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        if inserted.rows_affected() == 0 {
            let row = sqlx::query("SELECT message_id, fingerprint FROM mail_idempotency WHERE scope = $1 AND idempotency_key = $2")
                .bind(&request.idempotency_scope)
                .bind(&request.idempotency_key)
                .fetch_one(&mut *transaction)
                .await?;
            let existing: Vec<u8> = row.try_get("fingerprint")?;
            if existing != fingerprint {
                return Err(QueueError::IdempotencyConflict);
            }
            let existing_id: MessageId = row.try_get("message_id")?;
            transaction.rollback().await?;
            return Ok(EnqueueResult {
                message_id: existing_id,
                created: false,
            });
        }
        transaction.commit().await?;
        Ok(EnqueueResult {
            message_id,
            created: true,
        })
    }

    pub async fn claim_delivery_batch(
        &self,
        owner: &str,
    ) -> Result<Option<ClaimedDeliveryBatch>, QueueError> {
        self.claim_delivery_batch_mode(owner, false, None).await
    }

    pub async fn claim_relay_delivery_batch(
        &self,
        owner: &str,
    ) -> Result<Option<ClaimedDeliveryBatch>, QueueError> {
        self.claim_delivery_batch_mode(owner, true, None).await
    }

    /// Claims only messages for one explicitly configured transport. Workers
    /// use this to keep Mailpit, relay, and direct-MX queues isolated.
    pub async fn claim_delivery_batch_for_mode(
        &self,
        owner: &str,
        delivery_mode: &str,
    ) -> Result<Option<ClaimedDeliveryBatch>, QueueError> {
        let relay_mode = matches!(delivery_mode, "relay" | "mailpit");
        if !matches!(delivery_mode, "direct_mx" | "relay" | "mailpit") {
            return Err(QueueError::InvalidInput("unknown delivery mode".into()));
        }
        self.claim_delivery_batch_mode(owner, relay_mode, Some(delivery_mode)).await
    }

    async fn claim_delivery_batch_mode(
        &self,
        owner: &str,
        relay_mode: bool,
        delivery_mode: Option<&str>,
    ) -> Result<Option<ClaimedDeliveryBatch>, QueueError> {
        if owner.is_empty() || owner.len() > 128 {
            return Err(QueueError::InvalidInput(
                "delivery lease owner must be 1-128 bytes".into(),
            ));
        }
        self.recover_expired_delivery_leases().await?;
        self.fail_expired_deliveries().await?;

        let token = Uuid::new_v4();
        let lease_millis = i64::try_from(self.config.lease_duration.as_millis())
            .map_err(|_| QueueError::Configuration("lease duration is too large".into()))?;
        let max_age_millis = i64::try_from(self.config.max_delivery_age.as_millis())
            .map_err(|_| QueueError::Configuration("delivery age is too large".into()))?;
        let mut transaction = self.pool.begin().await?;
        let candidate = sqlx::query(
            "SELECT delivery.id, delivery.message_id, delivery.destination_domain
             FROM mail_recipient_deliveries AS delivery
             JOIN mail_queue AS queue ON queue.message_id = delivery.message_id
             JOIN mail_messages AS message ON message.id = delivery.message_id
             WHERE delivery.state IN ('pending', 'retry_scheduled')
               AND delivery.next_attempt_at <= NOW()
               AND ($3::text IS NULL OR message.delivery_mode = $3)
               AND queue.created_at > NOW() - ($1 * INTERVAL '1 millisecond')
               AND NOT EXISTS (
                   SELECT 1 FROM mail_delivery_domain_leases AS domain_lease
                   WHERE domain_lease.lease_expires_at > NOW()
                     AND (
                         domain_lease.destination_domain = CASE
                             WHEN $2 THEN 'relay:' || delivery.message_id::text
                             ELSE delivery.destination_domain
                         END
                         OR (NOT $2 AND domain_lease.destination_domain =
                             'relay:' || delivery.message_id::text)
                         OR ($2 AND EXISTS (
                             SELECT 1 FROM mail_recipient_deliveries AS same_message
                             WHERE same_message.message_id = delivery.message_id
                               AND same_message.destination_domain =
                                   domain_lease.destination_domain
                         ))
                     )
               )
             ORDER BY delivery.next_attempt_at, delivery.created_at, delivery.id
             FOR UPDATE OF queue SKIP LOCKED
             LIMIT 1",
        )
        .bind(max_age_millis)
        .bind(relay_mode)
        .bind(delivery_mode)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(candidate) = candidate else {
            transaction.commit().await?;
            return Ok(None);
        };
        let message_id: MessageId = candidate.try_get("message_id")?;
        let recipient_domain: String = candidate.try_get("destination_domain")?;
        let destination_domain = if relay_mode {
            format!("relay:{message_id}")
        } else {
            recipient_domain.clone()
        };
        let domain_lease = sqlx::query(
            "INSERT INTO mail_delivery_domain_leases
                (destination_domain, lease_owner, lease_token, lease_expires_at)
             VALUES ($1, $2, $3, NOW() + ($4 * INTERVAL '1 millisecond'))
             ON CONFLICT (destination_domain) DO UPDATE
             SET lease_owner = EXCLUDED.lease_owner,
                 lease_token = EXCLUDED.lease_token,
                 lease_expires_at = EXCLUDED.lease_expires_at
             WHERE mail_delivery_domain_leases.lease_expires_at <= NOW()
             RETURNING destination_domain",
        )
        .bind(&destination_domain)
        .bind(owner)
        .bind(token)
        .bind(lease_millis)
        .fetch_optional(&mut *transaction)
        .await?;
        if domain_lease.is_none() {
            transaction.rollback().await?;
            return Ok(None);
        }

        let claimed = sqlx::query(
            "WITH eligible AS (
                SELECT delivery.id
                FROM mail_recipient_deliveries AS delivery
                WHERE delivery.message_id = $1
                  AND ($2 OR delivery.destination_domain = $3)
                  AND delivery.state IN ('pending', 'retry_scheduled')
                  AND delivery.next_attempt_at <= NOW()
                FOR UPDATE SKIP LOCKED
            )
            UPDATE mail_recipient_deliveries AS delivery
            SET state = 'processing', lease_owner = $4, lease_token = $5,
                lease_expires_at = NOW() + ($6 * INTERVAL '1 millisecond'),
                attempt_count = delivery.attempt_count + 1, updated_at = NOW()
            FROM eligible
            WHERE delivery.id = eligible.id
            RETURNING delivery.id, delivery.recipient_id, delivery.attempt_count",
        )
        .bind(message_id)
        .bind(relay_mode)
        .bind(&recipient_domain)
        .bind(owner)
        .bind(token)
        .bind(lease_millis)
        .fetch_all(&mut *transaction)
        .await?;
        if claimed.is_empty() {
            transaction.rollback().await?;
            return Ok(None);
        }

        let recipient_ids = claimed
            .iter()
            .map(|row| row.try_get::<i64, _>("recipient_id"))
            .collect::<Result<Vec<_>, _>>()?;
        let delivery_ids = claimed
            .iter()
            .map(|row| row.try_get::<i64, _>("id"))
            .collect::<Result<Vec<_>, _>>()?;
        let attempt_numbers = claimed
            .iter()
            .map(|row| row.try_get::<i32, _>("attempt_count"))
            .collect::<Result<Vec<_>, _>>()?;
        for (delivery_id, attempt_number) in delivery_ids.iter().zip(&attempt_numbers) {
            sqlx::query(
                "INSERT INTO mail_recipient_attempts (delivery_id, attempt_number)
                 VALUES ($1, $2)",
            )
            .bind(delivery_id)
            .bind(attempt_number)
            .execute(&mut *transaction)
            .await?;
        }
        let addresses = sqlx::query(
            "SELECT id, recipient FROM mail_recipients
             WHERE id = ANY($1) ORDER BY id",
        )
        .bind(&recipient_ids)
        .fetch_all(&mut *transaction)
        .await?;
        let address_by_id = addresses
            .into_iter()
            .map(|row| {
                Ok((
                    row.try_get::<i64, _>("id")?,
                    row.try_get::<String, _>("recipient")?,
                ))
            })
            .collect::<Result<std::collections::HashMap<_, _>, sqlx::Error>>()?;
        let attempt_recipients = delivery_ids
            .iter()
            .zip(&recipient_ids)
            .zip(&attempt_numbers)
            .map(|((delivery_id, recipient_id), attempt_number)| {
                let address = address_by_id.get(recipient_id).cloned().ok_or_else(|| {
                    QueueError::InvalidInput("delivery recipient disappeared".into())
                })?;
                Ok(ClaimedRecipient {
                    delivery_id: *delivery_id,
                    address,
                    attempt_number: u32::try_from(*attempt_number).map_err(|_| {
                        QueueError::InvalidInput("delivery attempt count is invalid".into())
                    })?,
                })
            })
            .collect::<Result<Vec<_>, QueueError>>()?;

        let message = sqlx::query(
            "SELECT tenant_id, envelope_sender, sender_domain, delivery_mode, dkim_key_ref,
                    body, message_schema_version, created_at
             FROM mail_messages WHERE id = $1",
        )
        .bind(message_id)
        .fetch_one(&mut *transaction)
        .await?;
        let tenant_id: Option<Uuid> = message.try_get("tenant_id")?;
        let envelope_sender: String = message.try_get("envelope_sender")?;
        let sender_domain: Option<String> = message.try_get("sender_domain")?;
        let delivery_mode: String = message.try_get("delivery_mode")?;
        let dkim_key_ref: Option<String> = message.try_get("dkim_key_ref")?;
        let raw_message: Vec<u8> = message.try_get("body")?;
        let schema_version: i16 = message.try_get("message_schema_version")?;
        let created_at: DateTime<Utc> = message.try_get("created_at")?;
        let recipients =
            sqlx::query("SELECT recipient FROM mail_recipients WHERE message_id = $1 ORDER BY id")
                .bind(message_id)
                .fetch_all(&mut *transaction)
                .await?
                .into_iter()
                .map(|row| row.try_get("recipient"))
                .collect::<Result<Vec<String>, sqlx::Error>>()?;
        sqlx::query(
            "UPDATE mail_queue
             SET state = 'delivery_processing', next_attempt_at = NULL,
                 lease_owner = NULL, lease_token = NULL, lease_expires_at = NULL,
                 updated_at = NOW()
             WHERE message_id = $1",
        )
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        let lease_expires_at = Utc::now()
            + chrono::Duration::from_std(self.config.lease_duration)
                .map_err(|_| QueueError::Configuration("invalid lease duration".into()))?;
        transaction.commit().await?;
        Ok(Some(ClaimedDeliveryBatch {
            message_id,
            tenant_id,
            envelope_sender,
            sender_domain,
            delivery_mode,
            dkim_key_ref,
            recipients,
            destination_domain,
            raw_message,
            schema_version: u16::try_from(schema_version).map_err(|_| {
                QueueError::InvalidInput("message schema version is invalid".into())
            })?,
            created_at,
            attempt_recipients,
            lease_token: token,
            lease_expires_at,
        }))
    }

    pub async fn complete_delivery_batch(
        &self,
        batch: &ClaimedDeliveryBatch,
        results: &[RecipientDeliveryResult],
    ) -> Result<(), QueueError> {
        if results.len() != batch.attempt_recipients.len() {
            return Err(QueueError::InvalidInput(
                "delivery result count does not match claimed recipients".into(),
            ));
        }
        let expected = batch
            .attempt_recipients
            .iter()
            .map(|recipient| (recipient.delivery_id, recipient.attempt_number))
            .collect::<std::collections::HashMap<_, _>>();
        let mut seen = std::collections::HashSet::with_capacity(results.len());
        if results.iter().any(|result| {
            !expected.contains_key(&result.delivery_id) || !seen.insert(result.delivery_id)
        }) {
            return Err(QueueError::InvalidInput(
                "delivery results contain unknown or duplicate recipients".into(),
            ));
        }

        let mut transaction = self.pool.begin().await?;
        for result in results {
            let attempt_number = expected[&result.delivery_id];
            let age_millis = (Utc::now() - batch.created_at).num_milliseconds().max(0);
            let retry = result.state == RecipientDeliveryState::TemporaryFailure
                && attempt_number < self.config.max_attempts
                && age_millis < self.config.max_delivery_age.as_millis() as i64;
            let next_attempt = if retry {
                let delay = retry::delay(&self.config, attempt_number, &mut rand::rng());
                Some(
                    Utc::now()
                        + chrono::Duration::from_std(delay)
                            .map_err(|_| QueueError::Configuration("invalid retry delay".into()))?,
                )
            } else {
                None
            };
            let state = match result.state {
                RecipientDeliveryState::AcceptedByRemote => "accepted_by_remote",
                RecipientDeliveryState::TemporaryFailure if retry => "retry_scheduled",
                RecipientDeliveryState::TemporaryFailure
                | RecipientDeliveryState::PermanentFailure => "permanently_failed",
                RecipientDeliveryState::Uncertain => "uncertain",
                RecipientDeliveryState::Suppressed => "suppressed",
            };
            let attempt_outcome = match state {
                "accepted_by_remote" => "accepted_by_remote",
                "retry_scheduled" => "temporary_failure",
                "permanently_failed" => "permanent_failure",
                "uncertain" => "uncertain",
                _ => "suppressed",
            };
            let diagnostic = result.diagnostic.as_deref().map(safe_diagnostic);
            let remote_host = result.remote_host.as_deref().map(safe_remote_host);
            let updated = sqlx::query(
                "UPDATE mail_recipient_deliveries
                 SET state = $1, next_attempt_at = $2, lease_owner = NULL,
                     lease_token = NULL, lease_expires_at = NULL,
                     last_smtp_code = $3, last_diagnostic = $4, last_remote_host = $5,
                     accepted_at = CASE WHEN $1 = 'accepted_by_remote' THEN NOW() ELSE accepted_at END,
                     updated_at = NOW()
                 WHERE id = $6 AND state = 'processing' AND lease_token = $7",
            )
            .bind(state)
            .bind(next_attempt)
            .bind(result.smtp_code.map(i16::try_from).transpose().map_err(|_| {
                QueueError::InvalidInput("SMTP status code is invalid".into())
            })?)
            .bind(diagnostic.as_deref())
            .bind(remote_host.as_deref())
            .bind(result.delivery_id)
            .bind(batch.lease_token)
            .execute(&mut *transaction)
            .await?;
            if updated.rows_affected() != 1 {
                return Err(QueueError::LeaseLost);
            }
            let attempt_updated = sqlx::query(
                "UPDATE mail_recipient_attempts
                 SET finished_at = NOW(), outcome = $1, smtp_code = $2,
                     diagnostic = $3, remote_host = $4
                 WHERE delivery_id = $5 AND attempt_number = $6
                   AND outcome = 'in_progress' AND finished_at IS NULL",
            )
            .bind(attempt_outcome)
            .bind(
                result
                    .smtp_code
                    .map(i16::try_from)
                    .transpose()
                    .map_err(|_| QueueError::InvalidInput("SMTP status code is invalid".into()))?,
            )
            .bind(diagnostic.as_deref())
            .bind(remote_host.as_deref())
            .bind(result.delivery_id)
            .bind(i32::try_from(attempt_number).map_err(|_| {
                QueueError::InvalidInput("delivery attempt count is invalid".into())
            })?)
            .execute(&mut *transaction)
            .await?;
            if attempt_updated.rows_affected() != 1 {
                return Err(QueueError::LeaseLost);
            }
        }
        sqlx::query(
            "DELETE FROM mail_delivery_domain_leases
             WHERE destination_domain = $1 AND lease_token = $2",
        )
        .bind(&batch.destination_domain)
        .bind(batch.lease_token)
        .execute(&mut *transaction)
        .await?;
        refresh_message_state(&mut transaction, batch.message_id).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn renew_delivery_batch_lease(
        &self,
        batch: &ClaimedDeliveryBatch,
    ) -> Result<bool, QueueError> {
        let lease_millis = i64::try_from(self.config.lease_duration.as_millis())
            .map_err(|_| QueueError::Configuration("lease duration is too large".into()))?;
        let expected_recipients = u64::try_from(batch.attempt_recipients.len())
            .map_err(|_| QueueError::InvalidInput("delivery batch is too large".into()))?;
        let mut transaction = self.pool.begin().await?;
        let domain_lease = sqlx::query(
            "UPDATE mail_delivery_domain_leases
             SET lease_expires_at = NOW() + ($1 * INTERVAL '1 millisecond')
             WHERE destination_domain = $2 AND lease_token = $3
               AND lease_expires_at > NOW()",
        )
        .bind(lease_millis)
        .bind(&batch.destination_domain)
        .bind(batch.lease_token)
        .execute(&mut *transaction)
        .await?;
        if domain_lease.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }

        let recipients = sqlx::query(
            "UPDATE mail_recipient_deliveries
             SET lease_expires_at = NOW() + ($1 * INTERVAL '1 millisecond'),
                 updated_at = NOW()
             WHERE message_id = $2 AND destination_domain = $3
               AND state = 'processing' AND lease_token = $4
               AND lease_expires_at > NOW()",
        )
        .bind(lease_millis)
        .bind(batch.message_id)
        .bind(&batch.destination_domain)
        .bind(batch.lease_token)
        .execute(&mut *transaction)
        .await?;
        if recipients.rows_affected() != expected_recipients {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn recover_expired_delivery_leases(&self) -> Result<u64, QueueError> {
        let mut transaction = self.pool.begin().await?;
        let affected = sqlx::query(
            "WITH expired AS (
                UPDATE mail_recipient_deliveries
                SET state = 'uncertain', next_attempt_at = NULL,
                    lease_owner = NULL, lease_token = NULL,
                    lease_expires_at = NULL,
                    last_diagnostic = 'worker lease expired; remote acceptance is unknown',
                    updated_at = NOW()
                WHERE state = 'processing' AND lease_expires_at <= NOW()
                RETURNING id, message_id, attempt_count
            ), finished AS (
                UPDATE mail_recipient_attempts AS attempt
                SET outcome = 'uncertain', finished_at = NOW(),
                    diagnostic = 'worker lease expired; remote acceptance is unknown'
                FROM expired
                WHERE attempt.delivery_id = expired.id
                  AND attempt.attempt_number = expired.attempt_count
                  AND attempt.outcome = 'in_progress'
                RETURNING expired.message_id
            )
            SELECT DISTINCT message_id FROM expired",
        )
        .fetch_all(&mut *transaction)
        .await?;
        sqlx::query("DELETE FROM mail_delivery_domain_leases WHERE lease_expires_at <= NOW()")
            .execute(&mut *transaction)
            .await?;
        for row in &affected {
            let message_id: MessageId = row.try_get("message_id")?;
            refresh_message_state(&mut transaction, message_id).await?;
        }
        transaction.commit().await?;
        Ok(affected.len() as u64)
    }

    async fn fail_expired_deliveries(&self) -> Result<u64, QueueError> {
        let age_millis = i64::try_from(self.config.max_delivery_age.as_millis())
            .map_err(|_| QueueError::Configuration("delivery age is too large".into()))?;
        let result = sqlx::query(
            "WITH expired AS (
                UPDATE mail_recipient_deliveries AS delivery
                SET state = 'permanently_failed',
                    next_attempt_at = NULL,
                    last_diagnostic = 'maximum delivery age exceeded',
                    updated_at = NOW()
                FROM mail_queue AS queue
                WHERE queue.message_id = delivery.message_id
                  AND delivery.state IN ('pending', 'retry_scheduled')
                  AND queue.created_at <= NOW() - ($1 * INTERVAL '1 millisecond')
                RETURNING delivery.message_id
            )
            SELECT DISTINCT message_id FROM expired",
        )
        .bind(age_millis)
        .fetch_all(&self.pool)
        .await?;
        for row in &result {
            let message_id: MessageId = row.try_get("message_id")?;
            let mut transaction = self.pool.begin().await?;
            refresh_message_state(&mut transaction, message_id).await?;
            transaction.commit().await?;
        }
        Ok(result.len() as u64)
    }

    pub async fn claim(&self, owner: &str) -> Result<Option<ClaimedJob>, QueueError> {
        let token = Uuid::new_v4();
        let lease_millis = self.config.lease_duration.as_millis() as i64;
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query(
            "WITH candidate AS (
                SELECT q.message_id FROM mail_queue q
                WHERE (q.state IN ('queued', 'retry_scheduled') AND q.next_attempt_at <= NOW())
                   OR (q.state = 'processing' AND q.lease_expires_at < NOW())
                ORDER BY q.next_attempt_at, q.created_at
                FOR UPDATE SKIP LOCKED LIMIT 1
            )
            UPDATE mail_queue q
            SET state = 'processing', lease_owner = $1, lease_token = $2,
                lease_expires_at = NOW() + ($3 * INTERVAL '1 millisecond'),
                attempt_count = q.attempt_count + 1, updated_at = NOW()
            FROM candidate c WHERE q.message_id = c.message_id
            RETURNING q.message_id, q.attempt_count, q.lease_expires_at",
        )
        .bind(owner)
        .bind(token)
        .bind(lease_millis)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let message_id: MessageId = row.try_get("message_id")?;
        let attempt_count: i32 = row.try_get("attempt_count")?;
        let lease_expires_at: DateTime<Utc> = row.try_get("lease_expires_at")?;
        let message = sqlx::query("SELECT envelope_sender, body FROM mail_messages WHERE id = $1")
            .bind(message_id)
            .fetch_one(&mut *transaction)
            .await?;
        let sender: String = message.try_get("envelope_sender")?;
        let body: Vec<u8> = message.try_get("body")?;
        let recipients =
            sqlx::query("SELECT recipient FROM mail_recipients WHERE message_id = $1 ORDER BY id")
                .bind(message_id)
                .fetch_all(&mut *transaction)
                .await?
                .into_iter()
                .map(|row| row.try_get("recipient"))
                .collect::<Result<Vec<String>, sqlx::Error>>()?;
        transaction.commit().await?;
        Ok(Some(ClaimedJob {
            message_id,
            sender,
            recipients,
            body,
            attempt_count: attempt_count as u32,
            lease_token: token,
            lease_expires_at,
        }))
    }

    pub async fn complete(
        &self,
        job: &ClaimedJob,
        outcome: Result<crate::TestProcessorOutcome, FailureCategory>,
    ) -> Result<(), QueueError> {
        let category = match outcome {
            Ok(crate::TestProcessorOutcome::Success) => None,
            Ok(crate::TestProcessorOutcome::TemporaryFailure) => Some(FailureCategory::Temporary),
            Ok(crate::TestProcessorOutcome::PermanentFailure) => Some(FailureCategory::Permanent),
            Err(category) => Some(category),
        };
        let mut transaction = self.pool.begin().await?;
        let updated = sqlx::query("UPDATE mail_queue SET state = $1, next_attempt_at = $2, lease_owner = NULL, lease_token = NULL, lease_expires_at = NULL, last_error_category = $3, last_error_detail = $4, updated_at = NOW() WHERE message_id = $5 AND lease_token = $6 AND state = 'processing'")
            .bind(next_state(&self.config, job, category.as_ref()))
            .bind(next_attempt_at(&self.config, job, category.as_ref()))
            .bind(category.as_ref().map(FailureCategory::as_str))
            .bind(category.as_ref().map(safe_detail))
            .bind(job.message_id)
            .bind(job.lease_token)
            .execute(&mut *transaction)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(QueueError::LeaseLost);
        }
        sqlx::query("INSERT INTO mail_attempts (message_id, attempt_number, started_at, finished_at, outcome, error_category, diagnostic) VALUES ($1, $2, NOW(), NOW(), $3, $4, $5)")
            .bind(job.message_id)
            .bind(job.attempt_count as i32)
            .bind(if category.is_none() { "simulated_success" } else { "failure" })
            .bind(category.as_ref().map(FailureCategory::as_str))
            .bind(category.as_ref().map(safe_detail))
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn recover_expired(&self) -> Result<u64, QueueError> {
        let result = sqlx::query("UPDATE mail_queue SET state = 'retry_scheduled', next_attempt_at = NOW(), lease_owner = NULL, lease_token = NULL, lease_expires_at = NULL, last_error_category = 'lease_lost', updated_at = NOW() WHERE state = 'processing' AND lease_expires_at < NOW()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn state(&self, message_id: MessageId) -> Result<Option<QueueState>, QueueError> {
        let row = sqlx::query("SELECT state FROM mail_queue WHERE message_id = $1")
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row
            .and_then(|row| row.try_get::<String, _>("state").ok())
            .and_then(|state| match state.as_str() {
                "queued" => Some(QueueState::Queued),
                "processing" => Some(QueueState::Processing),
                "retry_scheduled" => Some(QueueState::RetryScheduled),
                "simulated_succeeded" => Some(QueueState::SimulatedSucceeded),
                "terminal_failed" => Some(QueueState::TerminalFailed),
                "accepted_by_remote" => Some(QueueState::AcceptedByRemote),
                "partially_accepted" => Some(QueueState::PartiallyAccepted),
                "permanently_failed" => Some(QueueState::PermanentlyFailed),
                "cancelled" => Some(QueueState::Cancelled),
                "suppressed" => Some(QueueState::Suppressed),
                "delivery_uncertain" => Some(QueueState::DeliveryUncertain),
                "delivery_processing" => Some(QueueState::DeliveryProcessing),
                _ => None,
            }))
    }

    pub async fn extend_delivery_batch_lease(
        &self,
        batch: &ClaimedDeliveryBatch,
        extension: std::time::Duration,
    ) -> Result<(), QueueError> {
        let millis = i64::try_from(extension.as_millis())
            .map_err(|_| QueueError::Configuration("lease extension duration too large".into()))?;
        let mut transaction = self.pool.begin().await?;
        let domain_updated = sqlx::query(
            "UPDATE mail_delivery_domain_leases
             SET lease_expires_at = NOW() + ($1 * INTERVAL '1 millisecond')
             WHERE destination_domain = $2 AND lease_token = $3 AND lease_expires_at > NOW()",
        )
        .bind(millis)
        .bind(&batch.destination_domain)
        .bind(batch.lease_token)
        .execute(&mut *transaction)
        .await?;
        if domain_updated.rows_affected() != 1 {
            return Err(QueueError::LeaseLost);
        }

        let recipients_updated = sqlx::query(
            "UPDATE mail_recipient_deliveries
             SET lease_expires_at = NOW() + ($1 * INTERVAL '1 millisecond'), updated_at = NOW()
             WHERE message_id = $2 AND destination_domain = $3 AND lease_token = $4 AND state = 'processing' AND lease_expires_at > NOW()",
        )
        .bind(millis)
        .bind(batch.message_id)
        .bind(&batch.destination_domain)
        .bind(batch.lease_token)
        .execute(&mut *transaction)
        .await?;
        if recipients_updated.rows_affected() == 0 {
            return Err(QueueError::LeaseLost);
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn extend_job_lease(
        &self,
        message_id: MessageId,
        token: Uuid,
        extension: std::time::Duration,
    ) -> Result<(), QueueError> {
        let millis = i64::try_from(extension.as_millis())
            .map_err(|_| QueueError::Configuration("lease extension duration too large".into()))?;
        let updated = sqlx::query(
            "UPDATE mail_queue
             SET lease_expires_at = NOW() + ($1 * INTERVAL '1 millisecond'), updated_at = NOW()
             WHERE message_id = $2 AND lease_token = $3 AND state = 'processing' AND lease_expires_at > NOW()",
        )
        .bind(millis)
        .bind(message_id)
        .bind(token)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(QueueError::LeaseLost);
        }
        Ok(())
    }

    pub async fn list_dead_letters(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::model::DeadLetterItem>, QueueError> {
        let rows = sqlx::query(
            "SELECT q.message_id, m.envelope_sender, q.state, q.created_at, q.updated_at
             FROM mail_queue q
             JOIN mail_messages m ON m.id = q.message_id
             WHERE q.state IN ('permanently_failed', 'terminal_failed', 'delivery_uncertain', 'suppressed')
             ORDER BY q.updated_at DESC
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let message_id: MessageId = row.try_get("message_id")?;
            let envelope_sender: String = row.try_get("envelope_sender")?;
            let state_str: String = row.try_get("state")?;
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
            let state = match state_str.as_str() {
                "permanently_failed" => QueueState::PermanentlyFailed,
                "terminal_failed" => QueueState::TerminalFailed,
                "delivery_uncertain" => QueueState::DeliveryUncertain,
                "suppressed" => QueueState::Suppressed,
                _ => QueueState::PermanentlyFailed,
            };

            let recipient_rows = sqlx::query(
                "SELECT d.id, r.recipient, d.destination_domain, d.state, d.attempt_count,
                        d.last_smtp_code, d.last_diagnostic, d.last_remote_host
                 FROM mail_recipient_deliveries d
                 JOIN mail_recipients r ON r.id = d.recipient_id
                 WHERE d.message_id = $1
                 ORDER BY d.id",
            )
            .bind(message_id)
            .fetch_all(&self.pool)
            .await?;

            let recipient_deliveries = recipient_rows
                .into_iter()
                .map(|r| {
                    Ok(crate::model::DeadLetterRecipient {
                        delivery_id: r.try_get("id")?,
                        recipient: r.try_get("recipient")?,
                        destination_domain: r.try_get("destination_domain")?,
                        state: r.try_get("state")?,
                        attempt_count: r.try_get::<i32, _>("attempt_count")? as u32,
                        last_smtp_code: r
                            .try_get::<Option<i16>, _>("last_smtp_code")?
                            .map(|c| c as u16),
                        last_diagnostic: r.try_get("last_diagnostic")?,
                        last_remote_host: r.try_get("last_remote_host")?,
                    })
                })
                .collect::<Result<Vec<_>, sqlx::Error>>()?;

            items.push(crate::model::DeadLetterItem {
                message_id,
                envelope_sender,
                state,
                created_at,
                updated_at,
                recipient_deliveries,
            });
        }
        Ok(items)
    }

    pub async fn get_dead_letter(
        &self,
        message_id: MessageId,
    ) -> Result<Option<crate::model::DeadLetterItem>, QueueError> {
        let row = sqlx::query(
            "SELECT q.message_id, m.envelope_sender, q.state, q.created_at, q.updated_at
             FROM mail_queue q
             JOIN mail_messages m ON m.id = q.message_id
             WHERE q.message_id = $1
               AND q.state IN ('permanently_failed', 'terminal_failed', 'delivery_uncertain', 'suppressed')",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let envelope_sender: String = row.try_get("envelope_sender")?;
        let state_str: String = row.try_get("state")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        let state = match state_str.as_str() {
            "permanently_failed" => QueueState::PermanentlyFailed,
            "terminal_failed" => QueueState::TerminalFailed,
            "delivery_uncertain" => QueueState::DeliveryUncertain,
            "suppressed" => QueueState::Suppressed,
            _ => QueueState::PermanentlyFailed,
        };

        let recipient_rows = sqlx::query(
            "SELECT d.id, r.recipient, d.destination_domain, d.state, d.attempt_count,
                    d.last_smtp_code, d.last_diagnostic, d.last_remote_host
             FROM mail_recipient_deliveries d
             JOIN mail_recipients r ON r.id = d.recipient_id
             WHERE d.message_id = $1
             ORDER BY d.id",
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await?;

        let recipient_deliveries = recipient_rows
            .into_iter()
            .map(|r| {
                Ok(crate::model::DeadLetterRecipient {
                    delivery_id: r.try_get("id")?,
                    recipient: r.try_get("recipient")?,
                    destination_domain: r.try_get("destination_domain")?,
                    state: r.try_get("state")?,
                    attempt_count: r.try_get::<i32, _>("attempt_count")? as u32,
                    last_smtp_code: r
                        .try_get::<Option<i16>, _>("last_smtp_code")?
                        .map(|c| c as u16),
                    last_diagnostic: r.try_get("last_diagnostic")?,
                    last_remote_host: r.try_get("last_remote_host")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?;

        Ok(Some(crate::model::DeadLetterItem {
            message_id,
            envelope_sender,
            state,
            created_at,
            updated_at,
            recipient_deliveries,
        }))
    }

    pub async fn replay_dead_letter(&self, message_id: MessageId) -> Result<bool, QueueError> {
        let mut transaction = self.pool.begin().await?;
        let queue_row =
            sqlx::query("SELECT state FROM mail_queue WHERE message_id = $1 FOR UPDATE")
                .bind(message_id)
                .fetch_optional(&mut *transaction)
                .await?;

        let Some(row) = queue_row else {
            transaction.rollback().await?;
            return Ok(false);
        };

        let current_state: String = row.try_get("state")?;
        if !matches!(
            current_state.as_str(),
            "permanently_failed" | "terminal_failed" | "delivery_uncertain" | "suppressed"
        ) {
            transaction.rollback().await?;
            return Ok(false);
        }

        sqlx::query(
            "UPDATE mail_recipient_deliveries
             SET state = 'pending', next_attempt_at = NOW(), lease_owner = NULL,
                 lease_token = NULL, lease_expires_at = NULL, updated_at = NOW()
             WHERE message_id = $1
               AND state IN ('permanently_failed', 'uncertain', 'suppressed')",
        )
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "UPDATE mail_queue
             SET state = 'queued', next_attempt_at = NOW(), lease_owner = NULL,
                 lease_token = NULL, lease_expires_at = NULL, last_error_category = NULL,
                 last_error_detail = NULL, updated_at = NOW()
             WHERE message_id = $1",
        )
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(true)
    }

    pub async fn cancel_message(
        &self,
        message_id: MessageId,
        reason: &str,
    ) -> Result<bool, QueueError> {
        let mut transaction = self.pool.begin().await?;
        let queue_row =
            sqlx::query("SELECT state FROM mail_queue WHERE message_id = $1 FOR UPDATE")
                .bind(message_id)
                .fetch_optional(&mut *transaction)
                .await?;

        let Some(row) = queue_row else {
            transaction.rollback().await?;
            return Ok(false);
        };

        let current_state: String = row.try_get("state")?;
        if matches!(
            current_state.as_str(),
            "accepted_by_remote" | "simulated_succeeded" | "cancelled"
        ) {
            transaction.rollback().await?;
            return Ok(false);
        }

        let diag = safe_diagnostic(reason);
        sqlx::query(
            "UPDATE mail_recipient_deliveries
             SET state = 'suppressed', last_diagnostic = $1, lease_owner = NULL,
                 lease_token = NULL, lease_expires_at = NULL, updated_at = NOW()
             WHERE message_id = $2 AND state IN ('pending', 'processing', 'retry_scheduled', 'uncertain')",
        )
        .bind(&diag)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "UPDATE mail_queue
             SET state = 'cancelled', next_attempt_at = NULL, lease_owner = NULL,
                 lease_token = NULL, lease_expires_at = NULL, last_error_category = 'cancelled',
                 last_error_detail = $1, updated_at = NOW()
             WHERE message_id = $2",
        )
        .bind(&diag)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(true)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn fingerprint(request: &EnqueueRequest<'_>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    let envelope = request.canonical_message.envelope();
    hasher.update(envelope.sender.as_bytes());
    for recipient in &envelope.recipients {
        hasher.update([0]);
        hasher.update(recipient.as_bytes());
    }
    hasher.update([0]);
    hasher.update(request.canonical_message.raw_message());
    hasher.finalize().to_vec()
}

async fn refresh_message_state(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    message_id: MessageId,
) -> Result<(), QueueError> {
    sqlx::query(
        "UPDATE mail_queue AS queue
         SET state = CASE
             WHEN EXISTS (
                 SELECT 1 FROM mail_recipient_deliveries
                 WHERE message_id = $1 AND state = 'processing'
             ) THEN 'delivery_processing'
             WHEN EXISTS (
                 SELECT 1 FROM mail_recipient_deliveries
                 WHERE message_id = $1 AND state IN ('pending', 'retry_scheduled')
             ) THEN 'retry_scheduled'
             WHEN EXISTS (
                 SELECT 1 FROM mail_recipient_deliveries
                 WHERE message_id = $1 AND state = 'uncertain'
             ) THEN 'delivery_uncertain'
             WHEN EXISTS (
                 SELECT 1 FROM mail_recipient_deliveries
                 WHERE message_id = $1 AND state = 'accepted_by_remote'
             ) AND EXISTS (
                 SELECT 1 FROM mail_recipient_deliveries
                 WHERE message_id = $1 AND state IN ('permanently_failed', 'suppressed')
             ) THEN 'partially_accepted'
             WHEN EXISTS (
                 SELECT 1 FROM mail_recipient_deliveries
                 WHERE message_id = $1 AND state = 'accepted_by_remote'
             ) THEN 'accepted_by_remote'
             WHEN NOT EXISTS (
                 SELECT 1 FROM mail_recipient_deliveries
                 WHERE message_id = $1 AND state != 'suppressed'
             ) THEN 'suppressed'
             ELSE 'permanently_failed'
         END,
         next_attempt_at = (
             SELECT MIN(next_attempt_at) FROM mail_recipient_deliveries
             WHERE message_id = $1 AND state IN ('pending', 'retry_scheduled')
         ),
         lease_owner = NULL, lease_token = NULL, lease_expires_at = NULL,
         updated_at = NOW()
         WHERE queue.message_id = $1",
    )
    .bind(message_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn safe_diagnostic(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(512)
        .collect()
}

fn safe_remote_host(value: &str) -> String {
    value
        .trim_end_matches('.')
        .chars()
        .filter(|character| character.is_ascii_graphic())
        .take(253)
        .collect()
}

fn next_state(
    config: &QueueConfig,
    job: &ClaimedJob,
    category: Option<&FailureCategory>,
) -> &'static str {
    match category {
        None => "simulated_succeeded",
        Some(category) if retry::should_retry(category, job.attempt_count, config.max_attempts) => {
            "retry_scheduled"
        }
        Some(_) => "terminal_failed",
    }
}

fn next_attempt_at(
    config: &QueueConfig,
    job: &ClaimedJob,
    category: Option<&FailureCategory>,
) -> Option<DateTime<Utc>> {
    if category.is_some_and(|category| {
        retry::should_retry(category, job.attempt_count, config.max_attempts)
    }) {
        let mut rng = rand::rng();
        Some(
            Utc::now()
                + chrono::Duration::from_std(retry::delay(config, job.attempt_count, &mut rng))
                    .expect("bounded retry delay"),
        )
    } else {
        None
    }
}

fn safe_detail(category: &FailureCategory) -> &'static str {
    match category {
        FailureCategory::Temporary => "test processor temporary failure",
        FailureCategory::Permanent => "test processor permanent failure",
        FailureCategory::LeaseLost => "worker lease expired",
        FailureCategory::Cancelled => "worker cancelled",
        FailureCategory::Internal => "internal queue failure",
    }
}
