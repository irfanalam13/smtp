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
    #[error("idempotency key was reused with a different append request")]
    IdempotencyConflict,
    #[error("workflow value is invalid or exceeds its limit")]
    InvalidWorkflowValue,
    #[error("message metadata could not be encoded")]
    MessageMetadata,
    #[error("canonical message parsing failed: {0}")]
    MessageParsing(#[from] crate::mime::MimeError),
}

mod administration;
mod flags;
mod labels;
mod messages;
mod outbox;
mod retention;
mod rules;
mod submissions;

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
