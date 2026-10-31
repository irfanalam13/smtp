use super::*;

impl MailboxStore {
    pub async fn create_sent_submission(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        request: &CreateSentSubmission,
    ) -> Result<Uuid, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        if request
            .queue_id
            .as_deref()
            .is_some_and(|value| value.len() > 255)
        {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let id = Uuid::new_v4();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO mailbox_sent_submissions
                (id, mailbox_id, message_id, submission_id, idempotency_key, queue_id)
             SELECT $1, $2, $3, $4, $5, $6
             WHERE EXISTS (
                 SELECT 1 FROM mailbox_messages WHERE mailbox_id = $2 AND id = $3
             )
             ON CONFLICT (mailbox_id, idempotency_key) DO UPDATE
             SET updated_at = NOW()
             WHERE mailbox_sent_submissions.message_id = EXCLUDED.message_id
               AND mailbox_sent_submissions.submission_id = EXCLUDED.submission_id
             RETURNING id",
        )
        .bind(id)
        .bind(mailbox_id)
        .bind(request.message_id)
        .bind(request.submission_id)
        .bind(request.idempotency_key.as_slice())
        .bind(&request.queue_id)
        .fetch_optional(&self.pool)
        .await?;
        inserted.ok_or(MailboxStoreError::InvalidWorkflowState)
    }

    pub async fn record_sent_recipient_event(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        event: &SentRecipientEvent,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        if event.recipient_address.len() > 320
            || event
                .diagnostic
                .as_ref()
                .is_some_and(|value| value.len() > 1024)
            || !matches!(
                event.state.as_str(),
                "queued" | "accepted" | "deferred" | "delivered" | "bounced" | "failed"
            )
        {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let mut transaction = self.pool.begin().await?;
        let submission = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM mailbox_sent_submissions
             WHERE mailbox_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(mailbox_id)
        .bind(event.submission_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(MailboxStoreError::InvalidWorkflowState)?;
        let inserted = sqlx::query(
            "INSERT INTO mailbox_sent_recipient_events
                (id, submission_id, source_event_id, recipient_address, recipient_state,
                 status_code, diagnostic, event_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (source_event_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(submission)
        .bind(event.source_event_id)
        .bind(&event.recipient_address)
        .bind(&event.state)
        .bind(&event.status_code)
        .bind(&event.diagnostic)
        .bind(event.occurred_at)
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() == 0 {
            let existing = sqlx::query(
                "SELECT submission_id, recipient_address, recipient_state, status_code,
                        diagnostic, event_at
                 FROM mailbox_sent_recipient_events WHERE source_event_id = $1",
            )
            .bind(event.source_event_id)
            .fetch_optional(&mut *transaction)
            .await?;
            let Some(existing) = existing else {
                return Err(MailboxStoreError::InvalidWorkflowState);
            };
            let existing_at: DateTime<Utc> = existing.try_get("event_at")?;
            let same_timestamp = existing_at
                .signed_duration_since(event.occurred_at)
                .num_microseconds()
                .is_some_and(|difference| difference.abs() <= 1);
            let exact_duplicate = existing.try_get::<Uuid, _>("submission_id")? == submission
                && existing.try_get::<String, _>("recipient_address")? == event.recipient_address
                && existing.try_get::<String, _>("recipient_state")? == event.state
                && existing.try_get::<Option<String>, _>("status_code")? == event.status_code
                && existing.try_get::<Option<String>, _>("diagnostic")? == event.diagnostic
                && same_timestamp;
            if !exact_duplicate {
                return Err(MailboxStoreError::InvalidWorkflowState);
            }
            transaction.commit().await?;
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO mailbox_sent_recipient_status
                (submission_id, recipient_address, recipient_state, latest_event_at,
                 latest_source_event_id, status_code, diagnostic)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (submission_id, recipient_address) DO UPDATE
             SET recipient_state = EXCLUDED.recipient_state,
                 latest_event_at = EXCLUDED.latest_event_at,
                 latest_source_event_id = EXCLUDED.latest_source_event_id,
                 status_code = EXCLUDED.status_code,
                 diagnostic = EXCLUDED.diagnostic
             WHERE ROW(
                       mailbox_sent_recipient_status.latest_event_at,
                       mailbox_sent_recipient_status.latest_source_event_id
                   ) < ROW(EXCLUDED.latest_event_at, EXCLUDED.latest_source_event_id)
               AND mailbox_sent_recipient_status.recipient_state
                   NOT IN ('delivered', 'bounced', 'failed')",
        )
        .bind(submission)
        .bind(&event.recipient_address)
        .bind(&event.state)
        .bind(event.occurred_at)
        .bind(event.source_event_id)
        .bind(&event.status_code)
        .bind(&event.diagnostic)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE mailbox_sent_submissions AS submission
             SET submission_state = CASE
                 WHEN NOT EXISTS (
                     SELECT 1 FROM mailbox_sent_recipient_status AS recipient
                     WHERE recipient.submission_id = submission.id
                       AND recipient.recipient_state NOT IN ('delivered', 'bounced', 'failed')
                 ) THEN CASE WHEN EXISTS (
                     SELECT 1 FROM mailbox_sent_recipient_status AS recipient
                     WHERE recipient.submission_id = submission.id
                       AND recipient.recipient_state = 'delivered'
                 ) THEN 'completed' ELSE 'failed' END
                 WHEN EXISTS (
                     SELECT 1 FROM mailbox_sent_recipient_status AS recipient
                     WHERE recipient.submission_id = submission.id
                       AND recipient.recipient_state = 'deferred'
                 ) THEN 'deferred'
                 WHEN EXISTS (
                     SELECT 1 FROM mailbox_sent_recipient_status AS recipient
                     WHERE recipient.submission_id = submission.id
                       AND recipient.recipient_state = 'accepted'
                 ) THEN 'accepted'
                 ELSE 'queued'
             END,
             updated_at = NOW()
             WHERE submission.id = $1",
        )
        .bind(submission)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }
}
