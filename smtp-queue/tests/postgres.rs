use std::time::Duration;

use mail_queue::{
    EnqueueRequest, EnqueueResult, Queue, QueueConfig, RecipientDeliveryResult,
    RecipientDeliveryState, TestProcessorOutcome,
};
use shared_types::{CanonicalMessage, MessageEnvelope, MimeLimits};
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
        "TRUNCATE mail_attempts, mail_recipient_attempts, mail_delivery_domain_leases,
         mail_recipient_deliveries, mail_idempotency, mail_queue, mail_recipients, mail_messages",
    )
    .execute(queue.pool())
    .await
    .expect("clean isolated database");
}

fn canonical_message(body: &[u8]) -> CanonicalMessage {
    canonical_message_for_recipients(body, vec!["recipient@example.test".into()])
}

fn canonical_message_for_recipients(body: &[u8], recipients: Vec<String>) -> CanonicalMessage {
    let mut raw = b"From: header-sender@example.test\r\nTo: header-recipient@example.test\r\nSubject: queue test\r\n\r\n".to_vec();
    raw.extend_from_slice(body);
    CanonicalMessage::parse(
        MessageEnvelope {
            sender: "sender@example.test".into(),
            recipients,
        },
        raw,
        MimeLimits::default(),
    )
    .expect("parse canonical message")
}

async fn enqueue_recipients(queue: &Queue, key: &str, recipients: Vec<String>) -> EnqueueResult {
    let canonical = canonical_message_for_recipients(b"body", recipients);
    let request = EnqueueRequest::new(&canonical, key.into(), "integration-test".into());
    queue.enqueue(&request).await.unwrap()
}

async fn enqueue_body(
    queue: &Queue,
    key: &str,
    body: &[u8],
) -> Result<EnqueueResult, mail_queue::QueueError> {
    let canonical = canonical_message(body);
    let request = EnqueueRequest::new(&canonical, key.into(), "integration-test".into());
    queue.enqueue(&request).await
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn enqueue_is_atomic_and_idempotent() {
    let queue = queue().await;
    clean(&queue).await;
    let first = enqueue_body(&queue, "same", b"body").await.unwrap();
    let second = enqueue_body(&queue, "same", b"body").await.unwrap();
    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.message_id, second.message_id);
    assert!(matches!(
        enqueue_body(&queue, "same", b"different").await,
        Err(mail_queue::QueueError::IdempotencyConflict)
    ));
    let stored = sqlx::query(
        "SELECT body, message_schema_version, parsed_metadata
         FROM mail_messages WHERE id = $1",
    )
    .bind(first.message_id)
    .fetch_one(queue.pool())
    .await
    .unwrap();
    assert!(stored.get::<Vec<u8>, _>("body").ends_with(b"body"));
    assert_eq!(stored.get::<i16, _>("message_schema_version"), 1);
    assert_eq!(
        stored.get::<serde_json::Value, _>("parsed_metadata")["subject"],
        "queue test"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn concurrent_claims_and_stale_leases_are_safe() {
    let queue = queue().await;
    clean(&queue).await;
    let message = enqueue_body(&queue, "claim", b"body").await.unwrap();
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
    let message = enqueue_body(&queue, "retry", b"body").await.unwrap();
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
    let message = enqueue_body(&queue, "expired", b"body").await.unwrap();
    let first = queue.claim("crashed-worker").await.unwrap().unwrap();
    tokio::time::sleep(Duration::from_millis(75)).await;
    assert_eq!(queue.recover_expired().await.unwrap(), 1);
    let recovered = queue.claim("replacement-worker").await.unwrap().unwrap();
    assert_eq!(recovered.message_id, message.message_id);
    assert_ne!(first.lease_token, recovered.lease_token);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations through 000021"]
async fn delivery_batches_route_domains_and_persist_independent_recipient_outcomes() {
    let queue = queue().await;
    clean(&queue).await;
    let message = enqueue_recipients(
        &queue,
        "recipient-outcomes",
        vec![
            "first@alpha.example.test".into(),
            "second@alpha.example.test".into(),
            "third@beta.example.test".into(),
        ],
    )
    .await;

    let first = queue
        .claim_delivery_batch("delivery-test")
        .await
        .unwrap()
        .unwrap();
    assert!(
        first
            .attempt_recipients
            .iter()
            .all(|recipient| recipient.address.ends_with(&first.destination_domain))
    );
    assert_eq!(
        first.attempt_recipients.len(),
        if first.destination_domain == "alpha.example.test" {
            2
        } else {
            1
        }
    );
    queue
        .complete_delivery_batch(
            &first,
            &first
                .attempt_recipients
                .iter()
                .map(|recipient| RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state: RecipientDeliveryState::AcceptedByRemote,
                    smtp_code: Some(250),
                    diagnostic: Some("accepted".into()),
                    remote_host: Some("mx.example.test".into()),
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();

    let second = queue
        .claim_delivery_batch("delivery-test")
        .await
        .unwrap()
        .unwrap();
    assert_ne!(first.destination_domain, second.destination_domain);
    queue
        .complete_delivery_batch(
            &second,
            &second
                .attempt_recipients
                .iter()
                .map(|recipient| RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state: RecipientDeliveryState::TemporaryFailure,
                    smtp_code: Some(451),
                    diagnostic: Some("try later".into()),
                    remote_host: Some("mx.example.test".into()),
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::RetryScheduled)
    );
    tokio::time::sleep(Duration::from_millis(10)).await;
    let retry = queue
        .claim_delivery_batch("delivery-test")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retry.destination_domain, second.destination_domain);
    assert!(
        retry
            .attempt_recipients
            .iter()
            .all(|recipient| recipient.attempt_number == 2)
    );
    queue
        .complete_delivery_batch(
            &retry,
            &retry
                .attempt_recipients
                .iter()
                .map(|recipient| RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state: RecipientDeliveryState::PermanentFailure,
                    smtp_code: Some(550),
                    diagnostic: Some("recipient rejected".into()),
                    remote_host: Some("mx.example.test".into()),
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();

    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::PartiallyAccepted)
    );
    let persisted = sqlx::query(
        "SELECT state, COUNT(*) AS count
         FROM mail_recipient_deliveries
         WHERE message_id = $1 GROUP BY state",
    )
    .bind(message.message_id)
    .fetch_all(queue.pool())
    .await
    .unwrap();
    let state_counts = persisted
        .into_iter()
        .map(|row| (row.get::<String, _>("state"), row.get::<i64, _>("count")))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(state_counts.get("accepted_by_remote"), Some(&2));
    assert_eq!(state_counts.get("permanently_failed"), Some(&1));
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations through 000021"]
async fn relay_claim_batches_all_domains_for_one_message() {
    let queue = queue().await;
    clean(&queue).await;
    let message = enqueue_recipients(
        &queue,
        "relay-all-recipients",
        vec![
            "first@alpha.example.test".into(),
            "second@beta.example.test".into(),
            "third@gamma.example.test".into(),
        ],
    )
    .await;

    let batch = queue
        .claim_relay_delivery_batch("relay-worker")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(batch.message_id, message.message_id);
    assert!(batch.destination_domain.starts_with("relay:"));
    assert_eq!(batch.attempt_recipients.len(), 3);
    assert!(
        queue
            .claim_relay_delivery_batch("relay-worker-2")
            .await
            .unwrap()
            .is_none()
    );

    queue
        .complete_delivery_batch(
            &batch,
            &batch
                .attempt_recipients
                .iter()
                .map(|recipient| RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state: RecipientDeliveryState::AcceptedByRemote,
                    smtp_code: Some(250),
                    diagnostic: Some("accepted by SMTP relay".into()),
                    remote_host: Some("smtp.example.test".into()),
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::AcceptedByRemote)
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations through 000021"]
async fn expired_delivery_lease_is_uncertain_not_automatically_retried() {
    let queue = queue().await;
    clean(&queue).await;
    let message = enqueue_body(&queue, "uncertain", b"body").await.unwrap();
    let batch = queue
        .claim_delivery_batch("crashed-delivery-worker")
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(75)).await;

    assert_eq!(queue.recover_expired_delivery_leases().await.unwrap(), 1);
    assert!(!queue.renew_delivery_batch_lease(&batch).await.unwrap());
    assert!(
        queue
            .claim_delivery_batch("replacement-worker")
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        queue
            .complete_delivery_batch(
                &batch,
                &[RecipientDeliveryResult {
                    delivery_id: batch.attempt_recipients[0].delivery_id,
                    state: RecipientDeliveryState::AcceptedByRemote,
                    smtp_code: Some(250),
                    diagnostic: Some("stale worker".into()),
                    remote_host: Some("mx.example.test".into()),
                }]
            )
            .await,
        Err(mail_queue::QueueError::LeaseLost)
    ));
    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::DeliveryUncertain)
    );
    let attempt = sqlx::query("SELECT outcome FROM mail_recipient_attempts WHERE delivery_id = $1")
        .bind(batch.attempt_recipients[0].delivery_id)
        .fetch_one(queue.pool())
        .await
        .unwrap();
    assert_eq!(attempt.get::<String, _>("outcome"), "uncertain");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations through 000021"]
async fn delivery_lease_renewal_fences_and_extends_the_entire_batch() {
    let queue = Queue::connect(QueueConfig {
        database_url: test_database_url(),
        lease_duration: Duration::from_secs(2),
        ..QueueConfig::default()
    })
    .await
    .unwrap();
    clean(&queue).await;
    let message = enqueue_recipients(
        &queue,
        "lease-renewal",
        vec!["first@example.test".into(), "second@example.test".into()],
    )
    .await;
    let batch = queue
        .claim_delivery_batch("delivery-test")
        .await
        .unwrap()
        .unwrap();
    let before: Vec<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT lease_expires_at FROM mail_recipient_deliveries
         WHERE message_id = $1 ORDER BY recipient_id",
    )
    .bind(message.message_id)
    .fetch_all(queue.pool())
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(queue.renew_delivery_batch_lease(&batch).await.unwrap());
    let after: Vec<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT lease_expires_at FROM mail_recipient_deliveries
         WHERE message_id = $1 ORDER BY recipient_id",
    )
    .bind(message.message_id)
    .fetch_all(queue.pool())
    .await
    .unwrap();
    assert_eq!(before.len(), 2);
    assert_eq!(after.len(), before.len());
    assert!(
        after
            .iter()
            .zip(&before)
            .all(|(after, before)| after > before)
    );
    assert_eq!(queue.recover_expired_delivery_leases().await.unwrap(), 0);

    queue
        .complete_delivery_batch(
            &batch,
            &batch
                .attempt_recipients
                .iter()
                .map(|recipient| RecipientDeliveryResult {
                    delivery_id: recipient.delivery_id,
                    state: RecipientDeliveryState::AcceptedByRemote,
                    smtp_code: Some(250),
                    diagnostic: Some("accepted".into()),
                    remote_host: Some("mx.example.test".into()),
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations through 000021"]
async fn dead_letter_inspection_and_replay_workflow() {
    let queue = queue().await;
    clean(&queue).await;
    let message =
        enqueue_recipients(&queue, "dead-letter-test", vec!["fail@example.test".into()]).await;
    let batch = queue
        .claim_delivery_batch("worker-dlq")
        .await
        .unwrap()
        .unwrap();

    queue
        .complete_delivery_batch(
            &batch,
            &[RecipientDeliveryResult {
                delivery_id: batch.attempt_recipients[0].delivery_id,
                state: RecipientDeliveryState::PermanentFailure,
                smtp_code: Some(550),
                diagnostic: Some("mailbox not found".into()),
                remote_host: Some("mx.example.test".into()),
            }],
        )
        .await
        .unwrap();

    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::PermanentlyFailed)
    );

    let dead_letters = queue.list_dead_letters(10, 0).await.unwrap();
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(dead_letters[0].message_id, message.message_id);
    assert_eq!(dead_letters[0].recipient_deliveries.len(), 1);
    assert_eq!(
        dead_letters[0].recipient_deliveries[0].state,
        "permanently_failed"
    );
    assert_eq!(
        dead_letters[0].recipient_deliveries[0].last_smtp_code,
        Some(550)
    );

    let item = queue
        .get_dead_letter(message.message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.message_id, message.message_id);

    let replayed = queue.replay_dead_letter(message.message_id).await.unwrap();
    assert!(replayed);

    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::Queued)
    );

    let re_claimed = queue
        .claim_delivery_batch("worker-replay")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(re_claimed.message_id, message.message_id);
    assert_eq!(
        re_claimed.attempt_recipients[0].address,
        "fail@example.test"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations through 000021"]
async fn cancel_message_suppresses_pending_deliveries() {
    let queue = queue().await;
    clean(&queue).await;
    let message = enqueue_recipients(
        &queue,
        "cancel-test",
        vec!["cancel1@example.test".into(), "cancel2@example.test".into()],
    )
    .await;

    let cancelled = queue
        .cancel_message(message.message_id, "operator cancelled")
        .await
        .unwrap();
    assert!(cancelled);

    assert_eq!(
        queue.state(message.message_id).await.unwrap(),
        Some(mail_queue::QueueState::Cancelled)
    );

    let batch = queue.claim_delivery_batch("worker").await.unwrap();
    assert!(batch.is_none());
}
