use super::*;

impl MailboxStore {
    pub async fn claim_outbox_events(
        &self,
        worker_id: Uuid,
        limit: i64,
        lease_seconds: i64,
    ) -> Result<Vec<MailboxOutboxEvent>, MailboxStoreError> {
        if !(1..=500).contains(&limit) || !(1..=900).contains(&lease_seconds) {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let rows = sqlx::query(
            "WITH candidates AS (
                     SELECT id FROM mailbox_event_outbox
                     WHERE published_at IS NULL
                       AND available_at <= NOW()
                       AND (locked_until IS NULL OR locked_until < NOW())
                     ORDER BY available_at, created_at, id
                     FOR UPDATE SKIP LOCKED LIMIT $1
                 )
                 UPDATE mailbox_event_outbox AS event
                 SET locked_by = $2, locked_until = NOW() + make_interval(secs => $3),
                     attempts = attempts + 1
                 FROM candidates
                 WHERE event.id = candidates.id
                 RETURNING event.id, event.tenant_id, event.event_type,
                           event.idempotency_key, event.payload, event.attempts",
        )
        .bind(limit)
        .bind(worker_id)
        .bind(lease_seconds as f64)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(MailboxOutboxEvent {
                    id: row.try_get("id")?,
                    tenant_id: row.try_get("tenant_id")?,
                    event_type: row.try_get("event_type")?,
                    idempotency_key: row.try_get("idempotency_key")?,
                    payload: row.try_get("payload")?,
                    attempts: row.try_get("attempts")?,
                })
            })
            .collect()
    }

    pub async fn acknowledge_outbox_event(
        &self,
        worker_id: Uuid,
        event_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        let result = sqlx::query(
            "UPDATE mailbox_event_outbox
                 SET published_at = NOW(), locked_by = NULL, locked_until = NULL,
                     last_error = NULL
                 WHERE id = $1 AND locked_by = $2 AND published_at IS NULL",
        )
        .bind(event_id)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn retry_outbox_event(
        &self,
        worker_id: Uuid,
        event_id: Uuid,
        retry_after_seconds: i64,
        error: &str,
    ) -> Result<bool, MailboxStoreError> {
        if !(1..=86_400).contains(&retry_after_seconds) || error.len() > 1024 {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let result = sqlx::query(
            "UPDATE mailbox_event_outbox
                 SET available_at = NOW() + make_interval(secs => $3),
                     locked_by = NULL, locked_until = NULL, last_error = $4
                 WHERE id = $1 AND locked_by = $2 AND published_at IS NULL",
        )
        .bind(event_id)
        .bind(worker_id)
        .bind(retry_after_seconds as f64)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
