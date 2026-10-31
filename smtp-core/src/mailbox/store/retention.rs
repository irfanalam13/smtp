use super::*;

impl MailboxStore {
    pub async fn quarantine_message(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        request: &QuarantineRequest,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        if request.reason_code.is_empty()
            || request.reason_code.len() > 64
            || !request
                .reason_code
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let mut transaction = self.pool.begin().await?;
        let valid_junk_folder: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM mailbox_folders
                 WHERE id = $1 AND mailbox_id = $2 AND special_use = '\\Junk'
             )",
        )
        .bind(request.junk_folder_id)
        .bind(mailbox_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !valid_junk_folder {
            return Err(MailboxStoreError::FolderNotFound);
        }
        let row = sqlx::query(
            "SELECT folder_id FROM mailbox_messages
             WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL
             FOR UPDATE",
        )
        .bind(mailbox_id)
        .bind(request.message_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(MailboxStoreError::InvalidWorkflowState)?;
        let original_folder_id: Uuid = row.try_get("folder_id")?;
        if original_folder_id == request.junk_folder_id {
            transaction.commit().await?;
            return Ok(false);
        }
        let uid = allocate_uid(&mut transaction, request.junk_folder_id).await?;
        sqlx::query(
            "INSERT INTO mailbox_quarantine_entries
                (mailbox_id, message_id, original_folder_id, reason_code, review_until)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (mailbox_id, message_id) DO UPDATE
             SET original_folder_id = EXCLUDED.original_folder_id,
                 quarantine_state = 'quarantined', reason_code = EXCLUDED.reason_code,
                 quarantined_at = NOW(), review_until = EXCLUDED.review_until,
                 released_at = NULL, updated_at = NOW()",
        )
        .bind(mailbox_id)
        .bind(request.message_id)
        .bind(original_folder_id)
        .bind(&request.reason_code)
        .bind(request.review_until)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("UPDATE mailbox_messages SET folder_id = $1, uid = $2 WHERE id = $3")
            .bind(request.junk_folder_id)
            .bind(uid)
            .bind(request.message_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn release_quarantined_message(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        let mut transaction = self.pool.begin().await?;
        let entry = sqlx::query(
            "SELECT original_folder_id, quarantine_state
             FROM mailbox_quarantine_entries
             WHERE mailbox_id = $1 AND message_id = $2 FOR UPDATE",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(entry) = entry else {
            return Ok(false);
        };
        let state: String = entry.try_get("quarantine_state")?;
        if state != "quarantined" {
            transaction.commit().await?;
            return Ok(false);
        }
        let original_folder_id: Option<Uuid> = entry.try_get("original_folder_id")?;
        let Some(original_folder_id) = original_folder_id else {
            return Err(MailboxStoreError::FolderNotFound);
        };
        let uid = allocate_uid(&mut transaction, original_folder_id).await?;
        sqlx::query(
            "UPDATE mailbox_messages SET folder_id = $1, uid = $2
             WHERE mailbox_id = $3 AND id = $4",
        )
        .bind(original_folder_id)
        .bind(uid)
        .bind(mailbox_id)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE mailbox_quarantine_entries
             SET quarantine_state = 'released', released_at = NOW(), updated_at = NOW()
             WHERE mailbox_id = $1 AND message_id = $2",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn move_to_trash(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        trash_folder_id: Uuid,
        purge_after: Option<DateTime<Utc>>,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        let mut transaction = self.pool.begin().await?;
        let target_valid: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM mailbox_folders
                 WHERE id = $1 AND mailbox_id = $2 AND special_use = '\\Trash'
             )",
        )
        .bind(trash_folder_id)
        .bind(mailbox_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !target_valid {
            return Err(MailboxStoreError::FolderNotFound);
        }
        let row = sqlx::query(
            "SELECT folder_id, uid, flags FROM mailbox_messages
             WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL
             FOR UPDATE",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(MailboxStoreError::InvalidWorkflowState)?;
        let original_folder_id: Uuid = row.try_get("folder_id")?;
        if original_folder_id == trash_folder_id {
            transaction.commit().await?;
            return Ok(false);
        }
        let original_uid: i64 = row.try_get("uid")?;
        let mut flags: Vec<String> = row.try_get("flags")?;
        flags.retain(|flag| flag != "\\Deleted");
        let new_uid = allocate_uid(&mut transaction, trash_folder_id).await?;
        sqlx::query(
            "INSERT INTO mailbox_trash_entries
                (mailbox_id, message_id, original_folder_id, original_uid, purge_after)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (mailbox_id, message_id) DO UPDATE
             SET original_folder_id = EXCLUDED.original_folder_id,
                 original_uid = EXCLUDED.original_uid,
                 trashed_at = NOW(), purge_after = EXCLUDED.purge_after",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(original_folder_id)
        .bind(original_uid)
        .bind(purge_after)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE mailbox_messages SET folder_id = $1, uid = $2, flags = $3
             WHERE mailbox_id = $4 AND id = $5",
        )
        .bind(trash_folder_id)
        .bind(new_uid)
        .bind(flags)
        .bind(mailbox_id)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn restore_from_trash(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT original_folder_id FROM mailbox_trash_entries
             WHERE mailbox_id = $1 AND message_id = $2 FOR UPDATE",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let original_folder_id: Uuid = row.try_get("original_folder_id")?;
        let Some(new_uid) = sqlx::query_scalar::<_, i64>(
            "UPDATE mailbox_folders
             SET uid_next = uid_next + 1, updated_at = NOW()
             WHERE mailbox_id = $1 AND id = $2 AND uid_next < 9223372036854775807
             RETURNING uid_next - 1",
        )
        .bind(mailbox_id)
        .bind(original_folder_id)
        .fetch_optional(&mut *transaction)
        .await?
        else {
            return Err(MailboxStoreError::FolderNotFound);
        };
        sqlx::query(
            "UPDATE mailbox_messages SET folder_id = $1, uid = $2, deleted_at = NULL,
                    flags = array_remove(flags, '\\Deleted')
             WHERE mailbox_id = $3 AND id = $4",
        )
        .bind(original_folder_id)
        .bind(new_uid)
        .bind(mailbox_id)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("DELETE FROM mailbox_trash_entries WHERE mailbox_id = $1 AND message_id = $2")
            .bind(mailbox_id)
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn purge_expired_trash(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<u64, MailboxStoreError> {
        if !(1..=1000).contains(&limit) {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query(
            "WITH candidates AS (
                 SELECT trash.mailbox_id, trash.message_id
                 FROM mailbox_trash_entries AS trash
                 JOIN mailbox_messages AS message
                   ON message.mailbox_id = trash.mailbox_id
                  AND message.id = trash.message_id
                 LEFT JOIN mailbox_archive_entries AS archive
                   ON archive.mailbox_id = trash.mailbox_id
                  AND archive.message_id = trash.message_id
                 WHERE trash.purge_after <= $1
                   AND COALESCE(archive.legal_hold, FALSE) = FALSE
                 ORDER BY trash.purge_after, trash.mailbox_id, trash.message_id
                 FOR UPDATE OF trash SKIP LOCKED LIMIT $2
             ), removed AS (
                 DELETE FROM mailbox_messages AS message
                 USING candidates
                 WHERE message.mailbox_id = candidates.mailbox_id
                   AND message.id = candidates.message_id
                 RETURNING message.mailbox_id, message.raw_size
             ), totals AS (
                 SELECT mailbox_id, SUM(raw_size)::BIGINT AS raw_size, COUNT(*)::BIGINT AS count
                 FROM removed GROUP BY mailbox_id
             ), updated AS (
                 UPDATE mailboxes AS mailbox
                 SET used_bytes = GREATEST(mailbox.used_bytes - totals.raw_size, 0)
                 FROM totals
                 WHERE mailbox.id = totals.mailbox_id
                 RETURNING totals.count
             )
             SELECT COALESCE(SUM(count), 0)::BIGINT AS deleted_count FROM updated",
        )
        .bind(now)
        .bind(limit)
        .fetch_one(&mut *transaction)
        .await?;
        let count: i64 = rows.try_get("deleted_count")?;
        transaction.commit().await?;
        u64::try_from(count).map_err(|_| MailboxStoreError::InvalidQuotaUsage)
    }

    pub async fn save_draft_revision(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        expected_version: i64,
        raw_message: &[u8],
    ) -> Result<i64, MailboxStoreError> {
        validate_message_size(raw_message, self.max_message_bytes)?;
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        if expected_version < 0 {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let mut transaction = self.pool.begin().await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM mailbox_messages AS message
                 JOIN mailbox_folders AS folder
                   ON folder.mailbox_id = message.mailbox_id
                  AND folder.id = message.folder_id
                 WHERE message.mailbox_id = $1 AND message.id = $2
                   AND folder.special_use = '\\Drafts'
             )",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !exists {
            return Err(MailboxStoreError::InvalidWorkflowState);
        }
        sqlx::query(
            "INSERT INTO mailbox_drafts (mailbox_id, tenant_id, message_id, owner_user_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (mailbox_id, message_id) DO NOTHING",
        )
        .bind(mailbox_id)
        .bind(tenant_id)
        .bind(message_id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
        let current_version: i64 = sqlx::query_scalar(
            "SELECT current_version FROM mailbox_drafts
             WHERE mailbox_id = $1 AND message_id = $2 FOR UPDATE",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .fetch_one(&mut *transaction)
        .await?;
        if current_version != expected_version {
            return Err(MailboxStoreError::VersionConflict);
        }
        let next_version = current_version
            .checked_add(1)
            .ok_or(MailboxStoreError::InvalidWorkflowValue)?;
        sqlx::query(
            "INSERT INTO mailbox_draft_revisions
                (mailbox_id, tenant_id, message_id, version, raw_message, editor_user_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(mailbox_id)
        .bind(tenant_id)
        .bind(message_id)
        .bind(next_version)
        .bind(raw_message)
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE mailbox_drafts SET current_version = $3, updated_by = $4,
                    autosaved_at = NOW()
             WHERE mailbox_id = $1 AND message_id = $2",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(next_version)
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(next_version)
    }

    pub async fn prune_draft_revisions(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        keep_latest: i64,
    ) -> Result<u64, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        if !(1..=100).contains(&keep_latest) {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let result = sqlx::query(
            "DELETE FROM mailbox_draft_revisions
             WHERE mailbox_id = $1 AND message_id = $2
               AND version <= (
                   SELECT COALESCE(MAX(version), 0) - $3
                   FROM mailbox_draft_revisions
                   WHERE mailbox_id = $1 AND message_id = $2
               )",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(keep_latest)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn set_archive_policy(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        retention_until: Option<DateTime<Utc>>,
        legal_hold: bool,
    ) -> Result<(), MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        sqlx::query(
            "INSERT INTO mailbox_archive_entries
                (mailbox_id, message_id, retention_until, legal_hold)
             SELECT $1, $2, $3, $4
             WHERE EXISTS (
                 SELECT 1 FROM mailbox_messages WHERE mailbox_id = $1 AND id = $2
             )
             ON CONFLICT (mailbox_id, message_id) DO UPDATE
             SET retention_until = EXCLUDED.retention_until,
                 legal_hold = EXCLUDED.legal_hold, updated_at = NOW()",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(retention_until)
        .bind(legal_hold)
        .execute(&self.pool)
        .await?
        .rows_affected()
        .eq(&1)
        .then_some(())
        .ok_or(MailboxStoreError::InvalidWorkflowState)
    }
}
