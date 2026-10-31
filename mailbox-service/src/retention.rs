use std::time::Duration;

use sqlx::PgPool;

const PRUNE_BATCH_SIZE: i64 = 1000;

pub(crate) async fn run(pool: PgPool) {
    loop {
        match prune_expired_batch(&pool).await {
            Ok(0) => tokio::time::sleep(Duration::from_secs(3600)).await,
            Ok(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            Err(error) => {
                tracing::error!(%error, "mailbox append-idempotency cleanup failed");
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
