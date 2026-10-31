use super::*;

impl MailboxStore {
    pub async fn create_label(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        request: &CreateMailboxLabel,
    ) -> Result<MailboxLabel, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        if request.name.trim().is_empty()
            || request.name.len() > 64
            || !matches!(
                request.kind.as_str(),
                "label" | "category" | "tag" | "smart"
            )
            || request.color.as_deref().is_some_and(|value| {
                value.len() != 7
                    || !value.starts_with('#')
                    || !value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            || request
                .query_definition
                .as_ref()
                .is_some_and(|value| value.to_string().len() > 8192)
            || (request.kind == "smart") != request.query_definition.is_some()
        {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let row = sqlx::query(
            "INSERT INTO mailbox_labels
                (id, mailbox_id, name, label_kind, color, query_definition)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, mailbox_id, name, label_kind, color, query_definition",
        )
        .bind(Uuid::new_v4())
        .bind(mailbox_id)
        .bind(request.name.trim())
        .bind(&request.kind)
        .bind(&request.color)
        .bind(&request.query_definition)
        .fetch_one(&self.pool)
        .await?;
        label_from_row(&row)
    }

    pub async fn list_labels(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
    ) -> Result<Vec<MailboxLabel>, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Read,
        )
        .await?;
        let rows = sqlx::query(
            "SELECT id, mailbox_id, name, label_kind, color, query_definition
             FROM mailbox_labels WHERE mailbox_id = $1 ORDER BY lower(name), id",
        )
        .bind(mailbox_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(label_from_row).collect()
    }

    pub async fn assign_label(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        label_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        let result = sqlx::query(
            "INSERT INTO mailbox_message_label_assignments (mailbox_id, message_id, label_id)
             SELECT $1, $2, $3
             WHERE EXISTS (
                 SELECT 1 FROM mailbox_messages WHERE mailbox_id = $1 AND id = $2
             )
             ON CONFLICT DO NOTHING",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(label_id)
        .execute(&self.pool)
        .await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM mailbox_message_label_assignments
                 WHERE mailbox_id = $1 AND message_id = $2 AND label_id = $3
             )",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(label_id)
        .fetch_one(&self.pool)
        .await?;
        if !exists {
            return Err(MailboxStoreError::InvalidWorkflowState);
        }
        Ok(result.rows_affected() == 1)
    }

    pub async fn remove_label(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        message_id: Uuid,
        label_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::ReadWrite,
        )
        .await?;
        let result = sqlx::query(
            "DELETE FROM mailbox_message_label_assignments
             WHERE mailbox_id = $1 AND message_id = $2 AND label_id = $3",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(label_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
