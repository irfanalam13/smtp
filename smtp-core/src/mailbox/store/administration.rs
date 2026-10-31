use super::*;

impl MailboxStore {
    pub fn from_pool(pool: PgPool, max_message_bytes: usize) -> Self {
        Self {
            pool,
            max_message_bytes: max_message_bytes.min(crate::mime::MAX_CANONICAL_MESSAGE_BYTES),
        }
    }

    pub async fn create_mailbox(
        &self,
        request: &CreateMailbox,
    ) -> Result<MailboxRecord, MailboxStoreError> {
        self.create_mailbox_inner(request, None).await
    }

    pub async fn create_mailbox_for_user(
        &self,
        request: &CreateMailbox,
        actor_user_id: Uuid,
    ) -> Result<MailboxRecord, MailboxStoreError> {
        self.create_mailbox_inner(request, Some(actor_user_id))
            .await
    }

    async fn create_mailbox_inner(
        &self,
        request: &CreateMailbox,
        actor_user_id: Option<Uuid>,
    ) -> Result<MailboxRecord, MailboxStoreError> {
        validate_local_part(&request.local_part)?;
        if request.display_name.len() > 160 {
            return Err(MailboxStoreError::DisplayNameTooLong);
        }
        if request.quota_bytes <= 0 {
            return Err(MailboxStoreError::InvalidMailboxQuota);
        }

        let mut transaction = self.pool.begin().await?;
        if let Some(actor_user_id) = actor_user_id {
            let authorized = sqlx::query(
                "SELECT domain.id
                 FROM tenants AS tenant
                 JOIN users AS account
                   ON account.id = $2 AND account.status = 'active'
                 JOIN tenant_memberships AS membership
                   ON membership.tenant_id = tenant.id
                  AND membership.user_id = account.id
                  AND membership.role IN ('owner', 'admin')
                 JOIN domains AS domain
                   ON domain.id = $3 AND domain.tenant_id = tenant.id
                  AND domain.status = 'verified'
                 WHERE tenant.id = $1 AND tenant.status = 'active'
                 FOR SHARE OF tenant, account, membership, domain",
            )
            .bind(request.tenant_id)
            .bind(actor_user_id)
            .bind(request.domain_id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();
            if !authorized {
                return Err(MailboxStoreError::MailboxNotFound);
            }
            if request.kind == MailboxKind::User && request.owner_user_id != Some(actor_user_id) {
                let owner_is_member = match request.owner_user_id {
                    Some(owner_user_id) => sqlx::query(
                        "SELECT user_id FROM tenant_memberships
                             WHERE tenant_id = $1 AND user_id = $2
                             FOR SHARE",
                    )
                    .bind(request.tenant_id)
                    .bind(owner_user_id)
                    .fetch_optional(&mut *transaction)
                    .await?
                    .is_some(),
                    None => false,
                };
                if !owner_is_member {
                    return Err(MailboxStoreError::MailboxNotFound);
                }
            }
        }
        let mailbox_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mailboxes
                (id, tenant_id, domain_id, local_part, mailbox_kind, display_name,
                 owner_user_id, quota_bytes, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'provisioning')",
        )
        .bind(mailbox_id)
        .bind(request.tenant_id)
        .bind(request.domain_id)
        .bind(&request.local_part)
        .bind(request.kind.as_str())
        .bind(&request.display_name)
        .bind(request.owner_user_id)
        .bind(request.quota_bytes)
        .execute(&mut *transaction)
        .await?;

        ensure_default_folders_in_transaction(&mut transaction, mailbox_id).await?;
        let row = sqlx::query(
            "UPDATE mailboxes
             SET status = 'active', provisioned_at = NOW()
             WHERE id = $1 AND status = 'provisioning'
             RETURNING id, tenant_id, domain_id, local_part, mailbox_kind,
                       display_name, owner_user_id, status, quota_bytes, used_bytes",
        )
        .bind(mailbox_id)
        .fetch_one(&mut *transaction)
        .await?;
        if let Some(actor_user_id) = actor_user_id {
            sqlx::query(
                "INSERT INTO audit_events
                    (tenant_id, actor_user_id, action, target_type, target_id, metadata)
                 VALUES ($1, $2, 'mailbox.created', 'mailbox', $3, $4)",
            )
            .bind(request.tenant_id)
            .bind(actor_user_id)
            .bind(mailbox_id.to_string())
            .bind(serde_json::json!({
                "domain_id": request.domain_id,
                "kind": request.kind.as_str(),
                "owner_user_id": request.owner_user_id,
            }))
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        mailbox_from_row(&row)
    }

    pub async fn transition_mailbox(
        &self,
        mailbox_id: Uuid,
        target_status: MailboxStatus,
    ) -> Result<bool, MailboxStoreError> {
        let result = sqlx::query(
            "UPDATE mailboxes
             SET status = $2,
                 deleted_at = CASE WHEN $2 = 'deleting'
                                   THEN COALESCE(deleted_at, NOW()) ELSE deleted_at END
             WHERE id = $1 AND (
                 status = $2 OR
                 (status = 'active' AND $2 IN ('suspended', 'disabled', 'deleting')) OR
                 (status = 'suspended' AND $2 IN ('active', 'disabled', 'deleting')) OR
                 (status = 'disabled' AND $2 IN ('active', 'deleting'))
             )",
        )
        .bind(mailbox_id)
        .bind(target_status.as_str())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn add_alias(
        &self,
        tenant_id: Uuid,
        mailbox_id: Uuid,
        domain_id: Uuid,
        local_part: &str,
    ) -> Result<Uuid, MailboxStoreError> {
        validate_local_part(local_part)?;
        let alias_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mailbox_aliases (id, tenant_id, mailbox_id, domain_id, local_part)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(alias_id)
        .bind(tenant_id)
        .bind(mailbox_id)
        .bind(domain_id)
        .bind(local_part)
        .execute(&self.pool)
        .await?;
        Ok(alias_id)
    }

    pub async fn set_permission(
        &self,
        tenant_id: Uuid,
        mailbox_id: Uuid,
        user_id: Uuid,
        permission: MailboxPermission,
    ) -> Result<(), MailboxStoreError> {
        sqlx::query(
            "INSERT INTO mailbox_permissions (tenant_id, mailbox_id, user_id, permission)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (mailbox_id, user_id)
             DO UPDATE SET permission = EXCLUDED.permission, tenant_id = EXCLUDED.tenant_id",
        )
        .bind(tenant_id)
        .bind(mailbox_id)
        .bind(user_id)
        .bind(permission.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_permission(
        &self,
        tenant_id: Uuid,
        mailbox_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        let result = sqlx::query(
            "DELETE FROM mailbox_permissions
             WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3",
        )
        .bind(tenant_id)
        .bind(mailbox_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn create_folder(
        &self,
        mailbox_id: Uuid,
        name: &str,
    ) -> Result<MailboxFolder, MailboxStoreError> {
        crate::mailbox::validate_mailbox_name(name)?;
        let id = Uuid::new_v4();
        let uid_validity: i64 = sqlx::query_scalar("SELECT nextval('mailbox_uid_validity_seq')")
            .fetch_one(&self.pool)
            .await?;
        let row = sqlx::query(
            "INSERT INTO mailbox_folders (id, mailbox_id, name, uid_validity)
             VALUES ($1, $2, $3, $4)
             RETURNING id, mailbox_id, parent_id, name, special_use, uid_validity, uid_next",
        )
        .bind(id)
        .bind(mailbox_id)
        .bind(name)
        .bind(uid_validity)
        .fetch_one(&self.pool)
        .await?;
        folder_from_row(&row)
    }

    pub async fn create_child_folder(
        &self,
        parent_folder_id: Uuid,
        name: &str,
    ) -> Result<MailboxFolder, MailboxStoreError> {
        crate::mailbox::validate_mailbox_name(name)?;
        let parent = sqlx::query("SELECT mailbox_id FROM mailbox_folders WHERE id = $1 FOR UPDATE")
            .bind(parent_folder_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(MailboxStoreError::FolderNotFound)?;
        let mailbox_id: Uuid = parent.try_get("mailbox_id")?;
        let uid_validity: i64 = sqlx::query_scalar("SELECT nextval('mailbox_uid_validity_seq')")
            .fetch_one(&self.pool)
            .await?;
        let row = sqlx::query(
            "INSERT INTO mailbox_folders (id, mailbox_id, parent_id, name, uid_validity)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, mailbox_id, parent_id, name, special_use, uid_validity, uid_next",
        )
        .bind(Uuid::new_v4())
        .bind(mailbox_id)
        .bind(parent_folder_id)
        .bind(name)
        .bind(uid_validity)
        .fetch_one(&self.pool)
        .await?;
        folder_from_row(&row)
    }

    pub async fn ensure_default_folders(
        &self,
        mailbox_id: Uuid,
    ) -> Result<Vec<MailboxFolder>, MailboxStoreError> {
        let mut transaction = self.pool.begin().await?;
        let folders = ensure_default_folders_in_transaction(&mut transaction, mailbox_id).await?;
        transaction.commit().await?;
        Ok(folders)
    }

    pub async fn list_folders(
        &self,
        mailbox_id: Uuid,
    ) -> Result<Vec<MailboxFolder>, MailboxStoreError> {
        let rows = sqlx::query(
            "SELECT id, mailbox_id, parent_id, name, special_use, uid_validity, uid_next
             FROM mailbox_folders WHERE mailbox_id = $1 ORDER BY name",
        )
        .bind(mailbox_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(folder_from_row).collect()
    }
}
