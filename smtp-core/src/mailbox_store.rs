use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::mailbox::{
    MailboxError, MailboxFlag, MailboxMessage, validate_local_part, validate_message_size,
};
use crate::rules::{MailRule, RuleAction, RuleCondition, RuleContext, RuleDirection, RulePlanner};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxChange {
    pub token: i64,
    pub message_id: Uuid,
    pub kind: String,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxRuleRecord {
    pub id: Uuid,
    pub mailbox_id: Uuid,
    pub version: i64,
    pub direction: String,
    pub enabled: bool,
    pub priority: i32,
    pub stop_processing: bool,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxLabel {
    pub id: Uuid,
    pub mailbox_id: Uuid,
    pub name: String,
    pub kind: String,
    pub color: Option<String>,
    pub query_definition: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateMailboxLabel {
    pub name: String,
    pub kind: String,
    pub color: Option<String>,
    pub query_definition: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuarantineRequest {
    pub message_id: Uuid,
    pub junk_folder_id: Uuid,
    pub reason_code: String,
    pub review_until: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateSentSubmission {
    pub message_id: Uuid,
    pub submission_id: Uuid,
    pub idempotency_key: [u8; 32],
    pub queue_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SentRecipientEvent {
    pub submission_id: Uuid,
    pub source_event_id: Uuid,
    pub recipient_address: String,
    pub state: String,
    pub status_code: Option<String>,
    pub diagnostic: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxOutboxEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub idempotency_key: String,
    pub payload: serde_json::Value,
    pub attempts: i32,
}

#[derive(Clone)]
pub struct MailboxStore {
    pool: PgPool,
    max_message_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxFolder {
    pub id: Uuid,
    pub mailbox_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub special_use: Option<String>,
    pub uid_validity: u64,
    pub uid_next: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxKind {
    User,
    Shared,
}

impl MailboxKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Shared => "shared",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxStatus {
    Active,
    Suspended,
    Disabled,
    Deleting,
}

impl MailboxStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Disabled => "disabled",
            Self::Deleting => "deleting",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateMailbox {
    pub tenant_id: Uuid,
    pub domain_id: Uuid,
    pub local_part: String,
    pub display_name: String,
    pub owner_user_id: Option<Uuid>,
    pub kind: MailboxKind,
    pub quota_bytes: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub domain_id: Uuid,
    pub local_part: String,
    pub display_name: String,
    pub owner_user_id: Option<Uuid>,
    pub kind: MailboxKind,
    pub status: MailboxStatus,
    pub quota_bytes: i64,
    pub used_bytes: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxQuotaWarning {
    ApproachingLimit,
    NearLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MailboxQuota {
    pub used_bytes: i64,
    pub quota_bytes: i64,
    pub usage_percent: u8,
    pub warning: Option<MailboxQuotaWarning>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxPermission {
    Read,
    ReadWrite,
    Manage,
}

impl MailboxPermission {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::ReadWrite => "read_write",
            Self::Manage => "manage",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MailboxStoreError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Validation(#[from] MailboxError),
    #[error("mailbox folder not found")]
    FolderNotFound,
    #[error("mailbox not found")]
    MailboxNotFound,
    #[error("mailbox is not accepting new messages")]
    MailboxUnavailable,
    #[error("mailbox UID validity is invalid")]
    InvalidUidValidity,
    #[error("mailbox quota would be exceeded")]
    QuotaExceeded,
    #[error("mailbox UID space is exhausted")]
    UidExhausted,
    #[error("mailbox quota accounting value is invalid")]
    InvalidQuotaUsage,
    #[error("mailbox quota must be greater than zero")]
    InvalidMailboxQuota,
    #[error("mailbox display name exceeds 160 bytes")]
    DisplayNameTooLong,
    #[error("mailbox kind stored in the database is invalid")]
    InvalidMailboxKind,
    #[error("mailbox status stored in the database is invalid")]
    InvalidMailboxStatus,
    #[error("mailbox operation version is stale")]
    VersionConflict,
    #[error("mailbox message is not in the expected workflow state")]
    InvalidWorkflowState,
    #[error("workflow value is invalid or exceeds its limit")]
    InvalidWorkflowValue,
}

impl MailboxStore {
    pub fn from_pool(pool: PgPool, max_message_bytes: usize) -> Self {
        Self {
            pool,
            max_message_bytes,
        }
    }

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

    pub async fn create_mailbox(
        &self,
        request: &CreateMailbox,
    ) -> Result<MailboxRecord, MailboxStoreError> {
        validate_local_part(&request.local_part)?;
        if request.display_name.len() > 160 {
            return Err(MailboxStoreError::DisplayNameTooLong);
        }
        if request.quota_bytes <= 0 {
            return Err(MailboxStoreError::InvalidMailboxQuota);
        }

        let mut transaction = self.pool.begin().await?;
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

    pub async fn append(
        &self,
        folder_id: Uuid,
        raw_message: &[u8],
        flags: &[MailboxFlag],
        internal_date: DateTime<Utc>,
    ) -> Result<MailboxMessage, MailboxStoreError> {
        validate_message_size(raw_message, self.max_message_bytes)?;
        let parsed = crate::mime::parse_message(
            raw_message,
            crate::mime::MimeLimits {
                max_message_bytes: self.max_message_bytes,
                ..crate::mime::MimeLimits::default()
            },
        )
        .ok();
        let mut transaction = self.pool.begin().await?;
        let folder = sqlx::query("SELECT mailbox_id FROM mailbox_folders WHERE id = $1 FOR UPDATE")
            .bind(folder_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(MailboxStoreError::FolderNotFound)?;
        let mailbox_id: Uuid = folder.try_get("mailbox_id")?;
        let raw_size = i64::try_from(raw_message.len())
            .map_err(|_| MailboxStoreError::Validation(MailboxError::MessageTooLarge))?;
        let charged = sqlx::query_scalar::<_, Uuid>(
            "UPDATE mailboxes
             SET used_bytes = used_bytes + $1
             WHERE id = $2 AND status = 'active' AND deleted_at IS NULL
               AND $1 <= quota_bytes AND used_bytes <= quota_bytes - $1
             RETURNING id",
        )
        .bind(raw_size)
        .bind(mailbox_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if charged.is_none() {
            let row = sqlx::query("SELECT status, deleted_at FROM mailboxes WHERE id = $1")
                .bind(mailbox_id)
                .fetch_one(&mut *transaction)
                .await?;
            let status: String = row.try_get("status")?;
            let deleted_at: Option<DateTime<Utc>> = row.try_get("deleted_at")?;
            return if status != "active" || deleted_at.is_some() {
                Err(MailboxStoreError::MailboxUnavailable)
            } else {
                Err(MailboxStoreError::QuotaExceeded)
            };
        }
        let uid = sqlx::query_scalar::<_, i64>(
            "UPDATE mailbox_folders
             SET uid_next = uid_next + 1, updated_at = NOW()
             WHERE id = $1 AND uid_next < 9223372036854775807
             RETURNING uid_next - 1",
        )
        .bind(folder_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(MailboxStoreError::UidExhausted)?;
        let id = Uuid::new_v4();
        let flag_values = flags
            .iter()
            .map(MailboxFlag::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        let conversation_id = if let Some(parsed) = &parsed {
            let mut reference_ids = parsed
                .references
                .iter()
                .filter(|value| !value.is_empty() && value.len() <= 998)
                .cloned()
                .collect::<Vec<_>>();
            if let Some(in_reply_to) = &parsed.in_reply_to {
                reference_ids.extend(
                    in_reply_to
                        .split_whitespace()
                        .filter(|value| !value.is_empty() && value.len() <= 998)
                        .map(|value| value.trim_matches(['<', '>']).to_string()),
                );
            }
            reference_ids.dedup();
            if let Some(message_id_header) = parsed
                .message_id
                .as_deref()
                .filter(|value| !value.is_empty() && value.len() <= 998)
            {
                ensure_conversation_in_transaction(
                    &mut transaction,
                    mailbox_id,
                    message_id_header,
                    &reference_ids,
                )
                .await?
            } else {
                None
            }
        } else {
            None
        };
        let parsed_fields = parsed.as_ref().map(|message| {
            let recipients = message
                .to
                .iter()
                .chain(&message.cc)
                .cloned()
                .collect::<Vec<_>>();
            let attachment_names = message
                .attachments
                .iter()
                .filter_map(|attachment| attachment.filename.clone())
                .collect::<Vec<_>>();
            (
                message.from.first().cloned(),
                recipients,
                message
                    .to
                    .iter()
                    .chain(&message.cc)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" "),
                message.subject.clone(),
                message.body_text.join("\n"),
                attachment_names.clone(),
                attachment_names.join(" "),
                message.message_id.clone(),
                message.in_reply_to.clone(),
                message.references.clone(),
            )
        });
        sqlx::query(
            "INSERT INTO mailbox_messages
             (id, mailbox_id, folder_id, uid, raw_message, raw_size, flags, internal_date,
              sender_address, recipients, recipients_search_text, subject, body_text,
              attachment_names, attachment_names_search_text, header_message_id,
              in_reply_to, references_ids, conversation_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8,
                     $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)",
        )
        .bind(id)
        .bind(mailbox_id)
        .bind(folder_id)
        .bind(uid)
        .bind(raw_message)
        .bind(raw_size)
        .bind(&flag_values)
        .bind(internal_date)
        .bind(
            parsed_fields
                .as_ref()
                .and_then(|fields| fields.0.as_deref()),
        )
        .bind(
            parsed_fields
                .as_ref()
                .map(|fields| fields.1.as_slice())
                .unwrap_or(&[]),
        )
        .bind(
            parsed_fields
                .as_ref()
                .map(|fields| fields.2.as_str())
                .unwrap_or(""),
        )
        .bind(
            parsed_fields
                .as_ref()
                .and_then(|fields| fields.3.as_deref()),
        )
        .bind(parsed_fields.as_ref().map(|fields| fields.4.as_str()))
        .bind(
            parsed_fields
                .as_ref()
                .map(|fields| fields.5.as_slice())
                .unwrap_or(&[]),
        )
        .bind(
            parsed_fields
                .as_ref()
                .map(|fields| fields.6.as_str())
                .unwrap_or(""),
        )
        .bind(
            parsed_fields
                .as_ref()
                .and_then(|fields| fields.7.as_deref()),
        )
        .bind(
            parsed_fields
                .as_ref()
                .and_then(|fields| fields.8.as_deref()),
        )
        .bind(
            parsed_fields
                .as_ref()
                .map(|fields| fields.9.as_slice())
                .unwrap_or(&[]),
        )
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
        let mut stored_folder_id = folder_id;
        let mut stored_uid = uid;
        let mut stored_flags = flag_values;
        let rules = sqlx::query(
            "SELECT id, version, direction, enabled, priority, stop_processing,
                    conditions, actions
             FROM mailbox_filters
             WHERE mailbox_id = $1 AND enabled = TRUE
               AND direction IN ('incoming', 'both')
             ORDER BY priority, id",
        )
        .bind(mailbox_id)
        .fetch_all(&mut *transaction)
        .await?;
        let mut parsed_rules = Vec::with_capacity(rules.len());
        let mut rule_versions = Vec::with_capacity(rules.len());
        for row in rules {
            let id: Uuid = row.try_get("id")?;
            let version: i64 = row.try_get("version")?;
            let direction: String = row.try_get("direction")?;
            let direction = match direction.as_str() {
                "incoming" => RuleDirection::Incoming,
                "outgoing" => RuleDirection::Outgoing,
                "both" => RuleDirection::Both,
                _ => return Err(MailboxStoreError::InvalidWorkflowValue),
            };
            let condition: serde_json::Value = row.try_get("conditions")?;
            let actions: serde_json::Value = row.try_get("actions")?;
            parsed_rules.push(MailRule {
                id,
                direction,
                enabled: row.try_get("enabled")?,
                priority: row.try_get("priority")?,
                stop_processing: row.try_get("stop_processing")?,
                condition: serde_json::from_value::<RuleCondition>(condition)
                    .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?,
                actions: serde_json::from_value(actions)
                    .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?,
            });
            rule_versions.push((id, version));
        }
        let parsed_for_rules = parsed.as_ref();
        let from = parsed_for_rules
            .and_then(|message| message.from.first())
            .map(String::as_str)
            .unwrap_or_default();
        let to = parsed_for_rules
            .map(|message| message.to.join(" "))
            .unwrap_or_default();
        let cc = parsed_for_rules
            .map(|message| message.cc.join(" "))
            .unwrap_or_default();
        let subject = parsed_for_rules
            .and_then(|message| message.subject.as_deref())
            .unwrap_or_default();
        let body_preview = parsed_for_rules
            .map(|message| message.body_text.join("\n"))
            .unwrap_or_default()
            .chars()
            .take(16 * 1024)
            .collect::<String>();
        let plan = RulePlanner
            .plan(
                &parsed_rules,
                RuleContext {
                    direction: RuleDirection::Incoming,
                    from,
                    to: &to,
                    cc: &cc,
                    subject,
                    body_preview: &body_preview,
                    has_attachments: parsed_for_rules
                        .is_some_and(|message| !message.attachments.is_empty()),
                    size_bytes: raw_size as u64,
                },
            )
            .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?;
        for action in &plan.actions {
            match action {
                RuleAction::MoveToFolder(target_folder_id) => {
                    let target_exists: bool = sqlx::query_scalar(
                        "SELECT EXISTS (
                             SELECT 1 FROM mailbox_folders
                             WHERE id = $1 AND mailbox_id = $2
                         )",
                    )
                    .bind(target_folder_id)
                    .bind(mailbox_id)
                    .fetch_one(&mut *transaction)
                    .await?;
                    if !target_exists {
                        return Err(MailboxStoreError::FolderNotFound);
                    }
                    if *target_folder_id != stored_folder_id {
                        stored_uid = allocate_uid(&mut transaction, *target_folder_id).await?;
                        stored_folder_id = *target_folder_id;
                    }
                }
                RuleAction::AddLabel(label_id) => {
                    let assigned = sqlx::query(
                        "INSERT INTO mailbox_message_label_assignments
                            (mailbox_id, message_id, label_id)
                         SELECT $1, $2, $3
                         WHERE EXISTS (
                             SELECT 1 FROM mailbox_labels
                             WHERE mailbox_id = $1 AND id = $3 AND label_kind <> 'smart'
                         )
                         ON CONFLICT DO NOTHING",
                    )
                    .bind(mailbox_id)
                    .bind(id)
                    .bind(label_id)
                    .execute(&mut *transaction)
                    .await?;
                    let valid: bool = sqlx::query_scalar(
                        "SELECT EXISTS (
                             SELECT 1 FROM mailbox_labels
                             WHERE mailbox_id = $1 AND id = $2 AND label_kind <> 'smart'
                         )",
                    )
                    .bind(mailbox_id)
                    .bind(label_id)
                    .fetch_one(&mut *transaction)
                    .await?;
                    if !valid {
                        return Err(MailboxStoreError::InvalidWorkflowState);
                    }
                    let _ = assigned;
                }
                RuleAction::RemoveLabel(label_id) => {
                    sqlx::query(
                        "DELETE FROM mailbox_message_label_assignments
                         WHERE mailbox_id = $1 AND message_id = $2 AND label_id = $3",
                    )
                    .bind(mailbox_id)
                    .bind(id)
                    .bind(label_id)
                    .execute(&mut *transaction)
                    .await?;
                }
                RuleAction::MarkRead => {
                    if !stored_flags.iter().any(|flag| flag == "\\Seen") {
                        stored_flags.push("\\Seen".into());
                    }
                }
                RuleAction::MarkUnread => {
                    stored_flags.retain(|flag| flag != "\\Seen");
                }
                RuleAction::FlagImportant => {
                    if !stored_flags.iter().any(|flag| flag == "\\Flagged") {
                        stored_flags.push("\\Flagged".into());
                    }
                }
            }
        }
        if stored_folder_id != folder_id
            || stored_uid != uid
            || stored_flags
                != flags
                    .iter()
                    .map(|flag| flag.as_str().to_string())
                    .collect::<Vec<_>>()
        {
            sqlx::query(
                "UPDATE mailbox_messages SET folder_id = $1, uid = $2, flags = $3
                 WHERE mailbox_id = $4 AND id = $5",
            )
            .bind(stored_folder_id)
            .bind(stored_uid)
            .bind(&stored_flags)
            .bind(mailbox_id)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        }
        for rule_id in &plan.matched_rule_ids {
            let version = rule_versions
                .iter()
                .find(|(id, _)| id == rule_id)
                .map(|(_, version)| *version)
                .ok_or(MailboxStoreError::InvalidWorkflowValue)?;
            let mut hasher = Sha256::new();
            hasher.update(id.as_bytes());
            hasher.update(rule_id.as_bytes());
            hasher.update(version.to_be_bytes());
            let idempotency_key: [u8; 32] = hasher.finalize().into();
            sqlx::query(
                "INSERT INTO mailbox_rule_executions
                    (id, mailbox_id, filter_id, message_id, rule_version,
                     idempotency_key, execution_state, result, completed_at)
                 VALUES ($1, $2, $3, $4, $5, $6, 'applied', $7, NOW())
                 ON CONFLICT (mailbox_id, filter_id, idempotency_key) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(mailbox_id)
            .bind(rule_id)
            .bind(id)
            .bind(version)
            .bind(idempotency_key.as_slice())
            .bind(serde_json::json!({"action_count": plan.actions.len()}))
            .execute(&mut *transaction)
            .await?;
        }
        if let Some(conversation_id) = conversation_id {
            sqlx::query(
                "UPDATE mailbox_conversations
                 SET latest_message_at = GREATEST(latest_message_at, $3),
                     normalized_subject = COALESCE(normalized_subject, $4),
                     updated_at = NOW()
                 WHERE mailbox_id = $1 AND id = $2",
            )
            .bind(mailbox_id)
            .bind(conversation_id)
            .bind(internal_date)
            .bind(
                parsed_fields
                    .as_ref()
                    .and_then(|fields| fields.3.as_deref()),
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(MailboxMessage {
            id,
            folder_id: stored_folder_id,
            uid: u64::try_from(stored_uid).map_err(|_| MailboxStoreError::InvalidUidValidity)?,
            raw_message: raw_message.to_vec(),
            flags: mailbox_flags_from_values(&stored_flags),
        })
    }

    pub async fn list_messages(
        &self,
        folder_id: Uuid,
    ) -> Result<Vec<MailboxMessage>, MailboxStoreError> {
        let rows = sqlx::query(
            "SELECT id, folder_id, uid, raw_message, flags
             FROM mailbox_messages
             WHERE folder_id = $1 AND deleted_at IS NULL
             ORDER BY uid",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(message_from_row).collect()
    }

    pub async fn changes_since(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        token: i64,
        limit: i64,
    ) -> Result<Vec<MailboxChange>, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Read,
        )
        .await?;
        if token < 0 || !(1..=500).contains(&limit) {
            return Err(MailboxStoreError::InvalidWorkflowValue);
        }
        let rows = sqlx::query(
            "SELECT token, message_id, change_kind, payload, occurred_at
             FROM mailbox_change_log
             WHERE mailbox_id = $1 AND token > $2
             ORDER BY token LIMIT $3",
        )
        .bind(mailbox_id)
        .bind(token)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(MailboxChange {
                    token: row.try_get("token")?,
                    message_id: row.try_get("message_id")?,
                    kind: row.try_get("change_kind")?,
                    payload: row.try_get("payload")?,
                    occurred_at: row.try_get("occurred_at")?,
                })
            })
            .collect()
    }

    pub async fn mailbox_quota(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
    ) -> Result<MailboxQuota, MailboxStoreError> {
        let row = sqlx::query(
            "SELECT mailbox.used_bytes, mailbox.quota_bytes
             FROM mailboxes AS mailbox
             WHERE mailbox.id = $1 AND mailbox.tenant_id = $2
               AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
               AND (
                   mailbox.owner_user_id = $3 OR EXISTS (
                       SELECT 1 FROM mailbox_permissions AS permission
                       WHERE permission.mailbox_id = mailbox.id
                         AND permission.tenant_id = mailbox.tenant_id
                         AND permission.user_id = $3
                         AND permission.permission IN ('read', 'read_write', 'manage')
                   )
               )",
        )
        .bind(mailbox_id)
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MailboxStoreError::InvalidWorkflowState)?;
        mailbox_quota_from_row(&row)
    }

    pub async fn set_mailbox_quota(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        quota_bytes: i64,
    ) -> Result<MailboxQuota, MailboxStoreError> {
        if quota_bytes <= 0 {
            return Err(MailboxStoreError::InvalidMailboxQuota);
        }
        let row = sqlx::query(
            "UPDATE mailboxes AS mailbox
             SET quota_bytes = $4
             WHERE mailbox.id = $1 AND mailbox.tenant_id = $2
               AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
               AND mailbox.used_bytes <= $4
               AND (
                   mailbox.owner_user_id = $3 OR EXISTS (
                       SELECT 1 FROM mailbox_permissions AS permission
                       WHERE permission.mailbox_id = mailbox.id
                         AND permission.tenant_id = mailbox.tenant_id
                         AND permission.user_id = $3
                         AND permission.permission = 'manage'
                   )
               )
             RETURNING mailbox.used_bytes, mailbox.quota_bytes",
        )
        .bind(mailbox_id)
        .bind(tenant_id)
        .bind(user_id)
        .bind(quota_bytes)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MailboxStoreError::InvalidWorkflowState)?;
        mailbox_quota_from_row(&row)
    }

    pub async fn save_rule(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        rule: &MailRule,
    ) -> Result<i64, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        RulePlanner
            .plan(
                std::slice::from_ref(rule),
                RuleContext {
                    direction: rule.direction,
                    from: "",
                    to: "",
                    cc: "",
                    subject: "",
                    body_preview: "",
                    has_attachments: false,
                    size_bytes: 0,
                },
            )
            .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?;
        let direction = match rule.direction {
            RuleDirection::Incoming => "incoming",
            RuleDirection::Outgoing => "outgoing",
            RuleDirection::Both => "both",
        };
        let condition = serde_json::to_value(&rule.condition)
            .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?;
        let actions = serde_json::to_value(&rule.actions)
            .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?;
        let mut transaction = self.pool.begin().await?;
        let version = sqlx::query_scalar::<_, i64>(
            "INSERT INTO mailbox_filters
                (id, mailbox_id, name, direction, enabled, priority, stop_processing,
                 conditions, actions, version)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)
             ON CONFLICT (mailbox_id, id) DO UPDATE
             SET direction = EXCLUDED.direction, enabled = EXCLUDED.enabled,
                 priority = EXCLUDED.priority, stop_processing = EXCLUDED.stop_processing,
                 conditions = EXCLUDED.conditions, actions = EXCLUDED.actions,
                 version = mailbox_filters.version + 1, updated_at = NOW()
             RETURNING version",
        )
        .bind(rule.id)
        .bind(mailbox_id)
        .bind(format!("rule-{}", rule.id))
        .bind(direction)
        .bind(rule.enabled)
        .bind(rule.priority)
        .bind(rule.stop_processing)
        .bind(&condition)
        .bind(&actions)
        .fetch_one(&mut *transaction)
        .await?;
        let snapshot = serde_json::json!({
            "direction": direction,
            "enabled": rule.enabled,
            "priority": rule.priority,
            "stop_processing": rule.stop_processing,
            "conditions": condition,
            "actions": actions,
        });
        sqlx::query(
            "INSERT INTO mailbox_filter_versions (mailbox_id, filter_id, version, snapshot)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(mailbox_id)
        .bind(rule.id)
        .bind(version)
        .bind(snapshot)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(version)
    }

    pub async fn list_rules(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
    ) -> Result<Vec<MailboxRuleRecord>, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Read,
        )
        .await?;
        let rows = sqlx::query(
            "SELECT id, mailbox_id, version, direction, enabled, priority, stop_processing,
                    conditions, actions
             FROM mailbox_filters WHERE mailbox_id = $1
             ORDER BY priority, id",
        )
        .bind(mailbox_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(MailboxRuleRecord {
                    id: row.try_get("id")?,
                    mailbox_id: row.try_get("mailbox_id")?,
                    version: row.try_get("version")?,
                    direction: row.try_get("direction")?,
                    enabled: row.try_get("enabled")?,
                    priority: row.try_get("priority")?,
                    stop_processing: row.try_get("stop_processing")?,
                    conditions: row.try_get("conditions")?,
                    actions: row.try_get("actions")?,
                })
            })
            .collect()
    }

    pub async fn delete_rule(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        rule_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        let result = sqlx::query("DELETE FROM mailbox_filters WHERE mailbox_id = $1 AND id = $2")
            .bind(mailbox_id)
            .bind(rule_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

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
             WHERE message.mailbox_id = $3 AND message.id = $4
               AND mailbox.id = message.mailbox_id AND mailbox.tenant_id = $1
               AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
               AND (
                   mailbox.owner_user_id = $2 OR EXISTS (
                       SELECT 1 FROM mailbox_permissions AS permission
                       WHERE permission.mailbox_id = mailbox.id
                         AND permission.tenant_id = mailbox.tenant_id
                         AND permission.user_id = $2
                         AND permission.permission IN ('read_write', 'manage')
                   )
               )",
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
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query("SELECT mailbox_id FROM mailbox_folders WHERE id = $1 FOR UPDATE")
            .bind(folder_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(MailboxStoreError::FolderNotFound)?;
        let mailbox_id: Uuid = row.try_get("mailbox_id")?;
        let deleted = sqlx::query(
            "WITH removed AS (
                     DELETE FROM mailbox_messages
                     WHERE folder_id = $1 AND deleted_at IS NOT NULL
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

async fn ensure_conversation_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    mailbox_id: Uuid,
    message_id_header: &str,
    references: &[String],
) -> Result<Option<Uuid>, MailboxStoreError> {
    let mut lookup_headers = references.to_vec();
    lookup_headers.push(message_id_header.to_string());
    let rows = sqlx::query(
        "SELECT DISTINCT conversation_id
         FROM mailbox_messages
         WHERE mailbox_id = $1 AND header_message_id = ANY($2)
           AND conversation_id IS NOT NULL
         ORDER BY conversation_id",
    )
    .bind(mailbox_id)
    .bind(&lookup_headers)
    .fetch_all(&mut **transaction)
    .await?;
    let mut conversation_ids = rows
        .iter()
        .map(|row| row.try_get::<Uuid, _>("conversation_id"))
        .collect::<Result<Vec<_>, _>>()?;
    conversation_ids.dedup();
    let Some(conversation_id) = conversation_ids.first().copied() else {
        let conversation_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mailbox_conversations (mailbox_id, id)
             VALUES ($1, $2)",
        )
        .bind(mailbox_id)
        .bind(conversation_id)
        .execute(&mut **transaction)
        .await?;
        return Ok(Some(conversation_id));
    };

    for duplicate_id in conversation_ids.iter().skip(1) {
        sqlx::query(
            "UPDATE mailbox_messages
             SET conversation_id = $1
             WHERE mailbox_id = $2 AND conversation_id = $3",
        )
        .bind(conversation_id)
        .bind(mailbox_id)
        .bind(duplicate_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "UPDATE mailbox_conversations AS primary_conversation
             SET latest_message_at = GREATEST(
                     primary_conversation.latest_message_at,
                     duplicate_conversation.latest_message_at
                 ),
                 updated_at = NOW()
             FROM mailbox_conversations AS duplicate_conversation
             WHERE primary_conversation.mailbox_id = $1
               AND primary_conversation.id = $2
               AND duplicate_conversation.mailbox_id = $1
               AND duplicate_conversation.id = $3",
        )
        .bind(mailbox_id)
        .bind(conversation_id)
        .bind(duplicate_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "DELETE FROM mailbox_conversations
             WHERE mailbox_id = $1 AND id = $2",
        )
        .bind(mailbox_id)
        .bind(duplicate_id)
        .execute(&mut **transaction)
        .await?;
    }

    Ok(Some(conversation_id))
}

async fn authorize_mailbox(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    mailbox_id: Uuid,
    permission: MailboxPermission,
) -> Result<(), MailboxStoreError> {
    let allowed_permission = match permission {
        MailboxPermission::Read => ["read", "read_write", "manage"].as_slice(),
        MailboxPermission::ReadWrite => ["read_write", "manage"].as_slice(),
        MailboxPermission::Manage => ["manage"].as_slice(),
    };
    let allowed: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM mailboxes AS mailbox
             WHERE mailbox.id = $1 AND mailbox.tenant_id = $2
               AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
               AND (
                   mailbox.owner_user_id = $3 OR EXISTS (
                       SELECT 1 FROM mailbox_permissions AS permission
                       WHERE permission.mailbox_id = mailbox.id
                         AND permission.tenant_id = mailbox.tenant_id
                         AND permission.user_id = $3
                         AND permission.permission = ANY($4)
                   )
               )
         )",
    )
    .bind(mailbox_id)
    .bind(tenant_id)
    .bind(user_id)
    .bind(allowed_permission)
    .fetch_one(pool)
    .await?;
    if allowed {
        Ok(())
    } else {
        Err(MailboxStoreError::InvalidWorkflowState)
    }
}

async fn allocate_uid(
    transaction: &mut Transaction<'_, Postgres>,
    folder_id: Uuid,
) -> Result<i64, MailboxStoreError> {
    sqlx::query_scalar::<_, i64>(
        "UPDATE mailbox_folders
         SET uid_next = uid_next + 1, updated_at = NOW()
         WHERE id = $1 AND uid_next < 9223372036854775807
         RETURNING uid_next - 1",
    )
    .bind(folder_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(MailboxStoreError::UidExhausted)
}

async fn ensure_default_folders_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    mailbox_id: Uuid,
) -> Result<Vec<MailboxFolder>, MailboxStoreError> {
    const DEFAULTS: [(&str, Option<&str>); 6] = [
        ("INBOX", None),
        ("Sent", Some("\\Sent")),
        ("Drafts", Some("\\Drafts")),
        ("Trash", Some("\\Trash")),
        ("Archive", Some("\\Archive")),
        ("Junk", Some("\\Junk")),
    ];

    let mailbox_exists = sqlx::query("SELECT id FROM mailboxes WHERE id = $1 FOR UPDATE")
        .bind(mailbox_id)
        .fetch_optional(&mut **transaction)
        .await?
        .is_some();
    if !mailbox_exists {
        return Err(MailboxStoreError::MailboxNotFound);
    }

    let mut folders = Vec::with_capacity(DEFAULTS.len());
    for (name, special_use) in DEFAULTS {
        let uid_validity: i64 = sqlx::query_scalar("SELECT nextval('mailbox_uid_validity_seq')")
            .fetch_one(&mut **transaction)
            .await?;
        sqlx::query(
            "INSERT INTO mailbox_folders (id, mailbox_id, name, special_use, uid_validity)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(mailbox_id)
        .bind(name)
        .bind(special_use)
        .bind(uid_validity)
        .execute(&mut **transaction)
        .await?;
        let row = sqlx::query(
            "SELECT id, mailbox_id, parent_id, name, special_use, uid_validity, uid_next
             FROM mailbox_folders
             WHERE mailbox_id = $1
               AND (($2::TEXT IS NOT NULL AND special_use = $2)
                    OR ($2::TEXT IS NULL AND name = $3))",
        )
        .bind(mailbox_id)
        .bind(special_use)
        .bind(name)
        .fetch_one(&mut **transaction)
        .await?;
        folders.push(folder_from_row(&row)?);
    }
    Ok(folders)
}

fn mailbox_from_row(row: &sqlx::postgres::PgRow) -> Result<MailboxRecord, MailboxStoreError> {
    let kind: String = row.try_get("mailbox_kind")?;
    let kind = match kind.as_str() {
        "user" => MailboxKind::User,
        "shared" => MailboxKind::Shared,
        _ => return Err(MailboxStoreError::InvalidMailboxKind),
    };
    let status: String = row.try_get("status")?;
    let status = match status.as_str() {
        "active" => MailboxStatus::Active,
        "suspended" => MailboxStatus::Suspended,
        "disabled" => MailboxStatus::Disabled,
        "deleting" => MailboxStatus::Deleting,
        _ => return Err(MailboxStoreError::InvalidMailboxStatus),
    };
    Ok(MailboxRecord {
        id: row.try_get("id")?,
        tenant_id: row.try_get("tenant_id")?,
        domain_id: row.try_get("domain_id")?,
        local_part: row.try_get("local_part")?,
        display_name: row.try_get("display_name")?,
        owner_user_id: row.try_get("owner_user_id")?,
        kind,
        status,
        quota_bytes: row.try_get("quota_bytes")?,
        used_bytes: row.try_get("used_bytes")?,
    })
}

fn mailbox_quota_from_row(row: &sqlx::postgres::PgRow) -> Result<MailboxQuota, MailboxStoreError> {
    let used_bytes: i64 = row.try_get("used_bytes")?;
    let quota_bytes: i64 = row.try_get("quota_bytes")?;
    if used_bytes < 0 || quota_bytes <= 0 || used_bytes > quota_bytes {
        return Err(MailboxStoreError::InvalidQuotaUsage);
    }
    let usage_percent = u8::try_from((i128::from(used_bytes) * 100) / i128::from(quota_bytes))
        .map_err(|_| MailboxStoreError::InvalidQuotaUsage)?;
    let warning = if usage_percent >= 95 {
        Some(MailboxQuotaWarning::NearLimit)
    } else if usage_percent >= 80 {
        Some(MailboxQuotaWarning::ApproachingLimit)
    } else {
        None
    };
    Ok(MailboxQuota {
        used_bytes,
        quota_bytes,
        usage_percent,
        warning,
    })
}

fn folder_from_row(row: &sqlx::postgres::PgRow) -> Result<MailboxFolder, MailboxStoreError> {
    let uid_validity: i64 = row.try_get("uid_validity")?;
    let uid_next: i64 = row.try_get("uid_next")?;
    Ok(MailboxFolder {
        id: row.try_get("id")?,
        mailbox_id: row.try_get("mailbox_id")?,
        parent_id: row.try_get("parent_id")?,
        name: row.try_get("name")?,
        special_use: row.try_get("special_use")?,
        uid_validity: u64::try_from(uid_validity)
            .map_err(|_| MailboxStoreError::InvalidUidValidity)?,
        uid_next: u64::try_from(uid_next).map_err(|_| MailboxStoreError::InvalidUidValidity)?,
    })
}

fn message_from_row(row: &sqlx::postgres::PgRow) -> Result<MailboxMessage, MailboxStoreError> {
    let uid: i64 = row.try_get("uid")?;
    let raw_flags: Vec<String> = row.try_get("flags")?;
    Ok(MailboxMessage {
        id: row.try_get("id")?,
        folder_id: row.try_get("folder_id")?,
        uid: u64::try_from(uid).map_err(|_| MailboxStoreError::InvalidUidValidity)?,
        raw_message: row.try_get("raw_message")?,
        flags: mailbox_flags_from_values(&raw_flags),
    })
}

fn label_from_row(row: &sqlx::postgres::PgRow) -> Result<MailboxLabel, MailboxStoreError> {
    Ok(MailboxLabel {
        id: row.try_get("id")?,
        mailbox_id: row.try_get("mailbox_id")?,
        name: row.try_get("name")?,
        kind: row.try_get("label_kind")?,
        color: row.try_get("color")?,
        query_definition: row.try_get("query_definition")?,
    })
}

fn mailbox_flags_from_values(values: &[String]) -> Vec<MailboxFlag> {
    values
        .iter()
        .filter_map(|flag| match flag.as_str() {
            "\\Seen" => Some(MailboxFlag::Seen),
            "\\Answered" => Some(MailboxFlag::Answered),
            "\\Flagged" => Some(MailboxFlag::Flagged),
            "\\Deleted" => Some(MailboxFlag::Deleted),
            "\\Draft" => Some(MailboxFlag::Draft),
            "\\Recent" => Some(MailboxFlag::Recent),
            _ => None,
        })
        .collect()
}
