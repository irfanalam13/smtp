use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::{
    config::QueueConfig,
    error::QueueError,
    model::{ClaimedJob, EnqueueRequest, EnqueueResult, FailureCategory, MessageId, QueueState},
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

    pub async fn enqueue(&self, request: &EnqueueRequest) -> Result<EnqueueResult, QueueError> {
        request.validate(&self.config)?;
        let fingerprint = fingerprint(request);
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
        sqlx::query("INSERT INTO mail_messages (id, envelope_sender, body, body_size, created_at) VALUES ($1, $2, $3, $4, NOW())")
            .bind(message_id)
            .bind(&request.sender)
            .bind(&request.body)
            .bind(request.body.len() as i64)
            .execute(&mut *transaction)
            .await?;
        for recipient in &request.recipients {
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
                _ => None,
            }))
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn fingerprint(request: &EnqueueRequest) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(request.sender.as_bytes());
    for recipient in &request.recipients {
        hasher.update([0]);
        hasher.update(recipient.as_bytes());
    }
    hasher.update([0]);
    hasher.update(&request.body);
    hasher.finalize().to_vec()
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
