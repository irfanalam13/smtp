use std::time::Duration;

use mail_queue::{EnqueueRequest, Queue, QueueConfig, TestProcessorOutcome};
use sqlx::Row;

fn test_database_url() -> String {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("set TEST_DATABASE_URL to an isolated disposable PostgreSQL database");
    assert!(
        url.contains("test"),
        "TEST_DATABASE_URL must identify an isolated test database"
    );
    url
}

async fn queue() -> Queue {
    let config = QueueConfig {
        database_url: test_database_url(),
        initial_retry_delay: Duration::from_millis(1),
        max_retry_delay: Duration::from_millis(5),
        jitter: Duration::ZERO,
        lease_duration: Duration::from_millis(50),
        ..QueueConfig::default()
    };
    Queue::connect(config).await.expect("connect test database")
}

async fn clean(queue: &Queue) {
    sqlx::query(
        "TRUNCATE mail_attempts, mail_idempotency, mail_queue, mail_recipients, mail_messages",
    )
    .execute(queue.pool())
    .await
    .expect("clean isolated database");
}

fn request(key: &str, body: &[u8]) -> EnqueueRequest {
    EnqueueRequest {
        sender: "sender@example.test".into(),
        recipients: vec!["recipient@example.test".into()],
        body: body.to_vec(),
        idempotency_key: key.into(),
        idempotency_scope: "integration-test".into(),
    }
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn enqueue_is_atomic_and_idempotent() {
    let queue = queue().await;
    clean(&queue).await;
    let first = queue.enqueue(&request("same", b"body")).await.unwrap();
    let second = queue.enqueue(&request("same", b"body")).await.unwrap();
    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.message_id, second.message_id);
    assert!(matches!(
        queue.enqueue(&request("same", b"different")).await,
        Err(mail_queue::QueueError::IdempotencyConflict)
    ));
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn concurrent_claims_and_stale_leases_are_safe() {
    let queue = queue().await;
    clean(&queue).await;
    let message = queue.enqueue(&request("claim", b"body")).await.unwrap();
    let (first, second) = tokio::join!(queue.claim("worker-a"), queue.claim("worker-b"));
    let first = first.unwrap().unwrap();
    assert!(second.unwrap().is_none());
    assert_eq!(first.message_id, message.message_id);
    let mut stale = first.clone();
    stale.lease_token = uuid::Uuid::new_v4();
    assert!(matches!(
        queue
            .complete(&stale, Ok(TestProcessorOutcome::Success))
            .await,
        Err(mail_queue::QueueError::LeaseLost)
    ));
    queue
        .complete(&first, Ok(TestProcessorOutcome::Success))
        .await
        .unwrap();
    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::SimulatedSucceeded)
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn retries_and_attempt_history_are_persisted() {
    let queue = queue().await;
    clean(&queue).await;
    let message = queue.enqueue(&request("retry", b"body")).await.unwrap();
    let job = queue.claim("worker").await.unwrap().unwrap();
    queue
        .complete(&job, Ok(TestProcessorOutcome::TemporaryFailure))
        .await
        .unwrap();
    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::RetryScheduled)
    );
    tokio::time::sleep(Duration::from_millis(10)).await;
    let retry = queue.claim("worker").await.unwrap().unwrap();
    queue
        .complete(&retry, Ok(TestProcessorOutcome::PermanentFailure))
        .await
        .unwrap();
    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::TerminalFailed)
    );
    let attempts = sqlx::query("SELECT COUNT(*) AS count FROM mail_attempts WHERE message_id = $1")
        .bind(message.message_id)
        .fetch_one(queue.pool())
        .await
        .unwrap()
        .get::<i64, _>("count");
    assert_eq!(attempts, 2);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn expired_leases_are_recoverable() {
    let queue = queue().await;
    clean(&queue).await;
    let message = queue.enqueue(&request("expired", b"body")).await.unwrap();
    let first = queue.claim("crashed-worker").await.unwrap().unwrap();
    tokio::time::sleep(Duration::from_millis(75)).await;
    assert_eq!(queue.recover_expired().await.unwrap(), 1);
    let recovered = queue.claim("replacement-worker").await.unwrap().unwrap();
    assert_eq!(recovered.message_id, message.message_id);
    assert_ne!(first.lease_token, recovered.lease_token);
}
