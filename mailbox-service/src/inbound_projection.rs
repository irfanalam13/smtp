use std::time::Duration;

use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

pub(crate) async fn run(pool: PgPool) {
    loop {
        match project_next(&pool).await {
            Ok(true) => continue,
            Ok(false) => tokio::time::sleep(Duration::from_secs(1)).await,
            Err(error) => {
                tracing::error!(%error, "inbound mailbox projection failed");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

pub(crate) async fn project_next(pool: &PgPool) -> Result<bool, String> {
    project_next_for_tenant(pool, None).await
}

pub(crate) async fn project_next_for_tenant(
    pool: &PgPool,
    tenant_id: Option<Uuid>,
) -> Result<bool, String> {
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    let event = match sqlx::query(
        "SELECT id, tenant_id, mailbox_id, event_type, payload
         FROM mailbox_event_outbox
         WHERE event_type IN ('inbound.message.accepted', 'inbound.message.quarantined')
           AND ($1::UUID IS NULL OR tenant_id = $1)
           AND published_at IS NULL AND available_at <= NOW()
           AND (locked_until IS NULL OR locked_until < NOW())
         ORDER BY available_at, created_at, id
         FOR UPDATE SKIP LOCKED
         LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(event) => event,
        Err(error) => {
            let _ = transaction.rollback().await;
            return Err(error.to_string());
        }
    };
    let Some(event) = event else {
        transaction
            .commit()
            .await
            .map_err(|error| error.to_string())?;
        return Ok(false);
    };

    let event_id = event
        .try_get::<Uuid, _>("id")
        .map_err(|error| error.to_string())?;
    let result = project_event(&mut transaction, &event).await;
    match result {
        Ok(()) => {
            transaction
                .commit()
                .await
                .map_err(|error| error.to_string())?;
            Ok(true)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(|rollback_error| format!("{error}; rollback failed: {rollback_error}"))?;
            sqlx::query(
                "UPDATE mailbox_event_outbox
                 SET attempts = attempts + 1,
                     available_at = NOW() +
                         (LEAST(3600, power(2, LEAST(attempts, 11)))::INTEGER
                          * INTERVAL '1 second'),
                     locked_by = NULL, locked_until = NULL,
                     last_error = LEFT($2, 1024)
                 WHERE id = $1 AND published_at IS NULL",
            )
            .bind(event_id)
            .bind(&error)
            .execute(pool)
            .await
            .map_err(|retry_error| format!("{error}; retry scheduling failed: {retry_error}"))?;
            Err(error)
        }
    }
}

async fn project_event(
    transaction: &mut Transaction<'_, Postgres>,
    event: &sqlx::postgres::PgRow,
) -> Result<(), String> {
    let event_id: Uuid = event.try_get("id").map_err(|error| error.to_string())?;
    let tenant_id: Uuid = event
        .try_get("tenant_id")
        .map_err(|error| error.to_string())?;
    let mailbox_id: Uuid = event
        .try_get("mailbox_id")
        .map_err(|error| error.to_string())?;
    let event_type: String = event
        .try_get("event_type")
        .map_err(|error| error.to_string())?;
    let payload: serde_json::Value = event
        .try_get("payload")
        .map_err(|error| error.to_string())?;
    let message_id = payload
        .get("message_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| "inbound event has an invalid message ID".to_string())?;

    let existing_mailbox: Option<Uuid> =
        sqlx::query_scalar("SELECT mailbox_id FROM mailbox_messages WHERE id = $1")
            .bind(message_id)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|error| error.to_string())?;
    if let Some(existing_mailbox) = existing_mailbox {
        if existing_mailbox != mailbox_id {
            return Err("inbound message ID already belongs to another mailbox".into());
        }
        return acknowledge(transaction, event_id).await;
    }

    let inbound = sqlx::query(
        "SELECT tenant_id, mailbox_id, status, raw_message, raw_size, received_at,
                message_schema_version, parsed_metadata
         FROM inbound_messages WHERE id = $1",
    )
    .bind(message_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "inbound event references a missing message".to_string())?;
    let inbound_tenant: Uuid = inbound
        .try_get("tenant_id")
        .map_err(|error| error.to_string())?;
    let inbound_mailbox: Uuid = inbound
        .try_get("mailbox_id")
        .map_err(|error| error.to_string())?;
    let status: String = inbound
        .try_get("status")
        .map_err(|error| error.to_string())?;
    if inbound_tenant != tenant_id || inbound_mailbox != mailbox_id {
        return Err("inbound event tenant or mailbox does not match its message".into());
    }
    if status == "deleted" {
        return acknowledge(transaction, event_id).await;
    }
    let expected_status = match event_type.as_str() {
        "inbound.message.accepted" => "stored",
        "inbound.message.quarantined" => "quarantined",
        _ => return Err("unsupported inbound event type".into()),
    };
    if status != expected_status {
        return Err("inbound event type does not match message status".into());
    }

    let (special_use, fallback_name) = if status == "quarantined" {
        ("\\Junk", "Junk")
    } else {
        ("\\Inbox", "INBOX")
    };
    let folder = sqlx::query(
        "UPDATE mailbox_folders
         SET uid_next = uid_next + 1, updated_at = NOW()
         WHERE id = (
             SELECT id FROM mailbox_folders
             WHERE mailbox_id = $1
               AND (special_use = $2 OR (name = $3 AND parent_id IS NULL))
             ORDER BY (special_use = $2) DESC NULLS LAST, id
             LIMIT 1
         )
           AND uid_next < 9223372036854775807
         RETURNING id, uid_next - 1 AS uid",
    )
    .bind(mailbox_id)
    .bind(special_use)
    .bind(fallback_name)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "inbound target folder is missing or UID space is exhausted".to_string())?;
    let folder_id: Uuid = folder.try_get("id").map_err(|error| error.to_string())?;
    let uid: i64 = folder.try_get("uid").map_err(|error| error.to_string())?;

    sqlx::query(
        "INSERT INTO mailbox_messages
         (id, mailbox_id, folder_id, uid, raw_message, raw_size, internal_date,
          message_schema_version, parsed_metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(message_id)
    .bind(mailbox_id)
    .bind(folder_id)
    .bind(uid)
    .bind(
        inbound
            .try_get::<Vec<u8>, _>("raw_message")
            .map_err(|error| error.to_string())?,
    )
    .bind(
        inbound
            .try_get::<i64, _>("raw_size")
            .map_err(|error| error.to_string())?,
    )
    .bind(
        inbound
            .try_get::<chrono::DateTime<chrono::Utc>, _>("received_at")
            .map_err(|error| error.to_string())?,
    )
    .bind(
        inbound
            .try_get::<i16, _>("message_schema_version")
            .map_err(|error| error.to_string())?,
    )
    .bind(
        inbound
            .try_get::<serde_json::Value, _>("parsed_metadata")
            .map_err(|error| error.to_string())?,
    )
    .execute(&mut **transaction)
    .await
    .map_err(|error| error.to_string())?;

    acknowledge(transaction, event_id).await
}

async fn acknowledge(
    transaction: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
) -> Result<(), String> {
    let result = sqlx::query(
        "UPDATE mailbox_event_outbox
         SET published_at = NOW(), locked_by = NULL, locked_until = NULL, last_error = NULL
         WHERE id = $1 AND published_at IS NULL",
    )
    .bind(event_id)
    .execute(&mut **transaction)
    .await
    .map_err(|error| error.to_string())?;
    if result.rows_affected() != 1 {
        return Err("inbound event could not be acknowledged".into());
    }
    Ok(())
}
