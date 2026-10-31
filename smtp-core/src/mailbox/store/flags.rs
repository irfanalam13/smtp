use super::*;

impl MailboxStore {
    pub async fn set_flags(
        &self,
        message_id: Uuid,
        flags: &[MailboxFlag],
    ) -> Result<bool, MailboxStoreError> {
        let values = flags
            .iter()
            .map(MailboxFlag::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        let result = sqlx::query("UPDATE mailbox_messages SET flags = $1 WHERE id = $2")
            .bind(values)
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn set_message_flags(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        flags: &[MailboxFlag],
    ) -> Result<bool, MailboxStoreError> {
        let values = flags
            .iter()
            .map(MailboxFlag::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        let result = sqlx::query(
            "UPDATE mailbox_messages AS message
             SET flags = $5
             FROM mailboxes AS mailbox
             JOIN tenants AS tenant
               ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
             JOIN users AS account
               ON account.id = $2 AND account.status = 'active'
             JOIN tenant_memberships AS membership
               ON membership.tenant_id = mailbox.tenant_id
              AND membership.user_id = account.id
             WHERE message.mailbox_id = $3 AND message.id = $4
               AND mailbox.id = message.mailbox_id AND mailbox.tenant_id = $1
               AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
               AND (mailbox.owner_user_id = account.id OR EXISTS (
                       SELECT 1 FROM mailbox_permissions AS permission
                       WHERE permission.mailbox_id = mailbox.id
                         AND permission.tenant_id = mailbox.tenant_id
                         AND permission.user_id = account.id
                         AND permission.permission IN ('read_write', 'manage')
                   ))",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(mailbox_id)
        .bind(message_id)
        .bind(values)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(MailboxStoreError::InvalidWorkflowState);
        }
        Ok(true)
    }

    pub async fn mark_deleted(&self, message_id: Uuid) -> Result<bool, MailboxStoreError> {
        let result = sqlx::query(
            "UPDATE mailbox_messages
             SET deleted_at = COALESCE(deleted_at, NOW()),
                 flags = CASE WHEN '\\Deleted' = ANY(flags)
                              THEN flags ELSE array_append(flags, '\\Deleted') END
             WHERE id = $1",
        )
        .bind(message_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn expunge(&self, folder_id: Uuid) -> Result<u64, MailboxStoreError> {
        self.expunge_inner(folder_id, None).await
    }

    pub async fn expunge_for_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        folder_id: Uuid,
    ) -> Result<u64, MailboxStoreError> {
        self.expunge_inner(folder_id, Some((tenant_id, user_id)))
            .await
    }

    async fn expunge_inner(
        &self,
        folder_id: Uuid,
        actor: Option<(Uuid, Uuid)>,
    ) -> Result<u64, MailboxStoreError> {
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query("SELECT mailbox_id FROM mailbox_folders WHERE id = $1 FOR UPDATE")
            .bind(folder_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(MailboxStoreError::FolderNotFound)?;
        let mailbox_id: Uuid = row.try_get("mailbox_id")?;
        if let Some((tenant_id, user_id)) = actor {
            let access = sqlx::query(
                "SELECT mailbox.owner_user_id
                 FROM mailboxes AS mailbox
                 JOIN tenants AS tenant
                   ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
                 JOIN users AS account
                   ON account.id = $3 AND account.status = 'active'
                 JOIN tenant_memberships AS membership
                   ON membership.tenant_id = mailbox.tenant_id
                  AND membership.user_id = account.id
                 WHERE mailbox.id = $1 AND mailbox.tenant_id = $2
                   AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
                 FOR SHARE OF mailbox, tenant, account, membership",
            )
            .bind(mailbox_id)
            .bind(tenant_id)
            .bind(user_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(MailboxStoreError::MailboxNotFound)?;
            let owner_user_id: Option<Uuid> = access.try_get("owner_user_id")?;
            if owner_user_id != Some(user_id) {
                let permission = sqlx::query(
                    "SELECT permission FROM mailbox_permissions
                     WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3
                       AND permission IN ('read_write', 'manage')
                     FOR SHARE",
                )
                .bind(tenant_id)
                .bind(mailbox_id)
                .bind(user_id)
                .fetch_optional(&mut *transaction)
                .await?;
                if permission.is_none() {
                    return Err(MailboxStoreError::MailboxNotFound);
                }
            }
        }
        let deleted = sqlx::query(
            "WITH removed AS (
                     DELETE FROM mailbox_messages
                     WHERE folder_id = $1
                       AND (deleted_at IS NOT NULL OR '\\Deleted' = ANY(flags))
                     RETURNING raw_size
                 )
                 SELECT COALESCE(SUM(raw_size), 0)::BIGINT AS released_bytes,
                        COUNT(*)::BIGINT AS deleted_count
                 FROM removed",
        )
        .bind(folder_id)
        .fetch_one(&mut *transaction)
        .await?;
        let released_bytes: i64 = deleted.try_get("released_bytes")?;
        let deleted_count: i64 = deleted.try_get("deleted_count")?;
        sqlx::query("UPDATE mailboxes SET used_bytes = GREATEST(0, used_bytes - $1) WHERE id = $2")
            .bind(released_bytes)
            .bind(mailbox_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        u64::try_from(deleted_count).map_err(|_| MailboxStoreError::InvalidQuotaUsage)
    }
}
