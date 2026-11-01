use std::time::Duration;

use sqlx::PgPool;

const PRUNE_BATCH_SIZE: i64 = 1000;

pub(crate) async fn run(pool: PgPool) {
    loop {
        let idempotency = prune_expired_batch(&pool).await;
        let messages = prune_expired_messages_batch(&pool).await;
        match (idempotency, messages) {
            (Ok(0), Ok(0)) => tokio::time::sleep(Duration::from_secs(3600)).await,
            (Ok(_), Ok(_)) => tokio::time::sleep(Duration::from_millis(100)).await,
            (Ok(_), Err(error)) | (Err(error), Ok(_)) => {
                tracing::error!(%error, "mailbox retention cleanup failed");
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            (Err(error), Err(message_error)) => {
                tracing::error!(
                    %error,
                    %message_error,
                    "mailbox retention cleanup failed"
                );
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    }
}

pub(crate) async fn prune_expired_batch(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let rows = sqlx::query(
        "WITH expired AS (
             SELECT tenant_id, mailbox_id, user_id, idempotency_key
             FROM mailbox_append_idempotency
             WHERE expires_at <= NOW()
             ORDER BY expires_at
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
         DELETE FROM mailbox_append_idempotency AS key
         USING expired
         WHERE key.tenant_id = expired.tenant_id
           AND key.mailbox_id = expired.mailbox_id
           AND key.user_id = expired.user_id
           AND key.idempotency_key = expired.idempotency_key
         RETURNING key.idempotency_key",
    )
    .bind(PRUNE_BATCH_SIZE)
    .fetch_all(pool)
    .await?;
    Ok(rows.len() as u64)
}

pub(crate) async fn prune_expired_messages_batch(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let rows = sqlx::query(
        "WITH expired AS (
             SELECT archive.mailbox_id, archive.message_id
             FROM mailbox_archive_entries AS archive
             WHERE archive.retention_until <= NOW()
               AND archive.legal_hold = FALSE
             ORDER BY archive.retention_until, archive.mailbox_id, archive.message_id
             LIMIT $1
             FOR UPDATE OF archive SKIP LOCKED
         )
         DELETE FROM mailbox_messages AS message
         USING expired
         WHERE message.id = expired.message_id
           AND message.folder_id IN (
               SELECT folder.id
               FROM mailbox_folders AS folder
               WHERE folder.id = message.folder_id
                 AND folder.mailbox_id = expired.mailbox_id
           )
         RETURNING message.id",
    )
    .bind(PRUNE_BATCH_SIZE)
    .fetch_all(pool)
    .await?;
    Ok(rows.len() as u64)
}
