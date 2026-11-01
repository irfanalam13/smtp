use std::{sync::Arc, time::Duration};

use mail_delivery::{
    DeliveryConfig, DnsResolver, SmtpProcessor,
    config::{SmtpRelayConfig, SmtpRelayMode},
    worker::DeliveryWorker,
};
use mail_queue::{EnqueueRequest, Queue, QueueConfig, QueueState};
use shared_types::{CanonicalMessage, MessageEnvelope, MimeLimits};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

fn test_database_url() -> String {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("set TEST_DATABASE_URL to an isolated disposable PostgreSQL database");
    assert!(
        url.contains("test"),
        "TEST_DATABASE_URL must identify an isolated test database"
    );
    url
}

async fn clean(queue: &Queue) {
    sqlx::query(
        "TRUNCATE mail_attempts, mail_recipient_attempts, mail_delivery_domain_leases,
         mail_recipient_deliveries, mail_idempotency, mail_queue, mail_recipients, mail_messages CASCADE",
    )
    .execute(queue.pool())
    .await
    .expect("clean isolated database");
    sqlx::query("DELETE FROM tenants WHERE name = 'Test Tenant'")
        .execute(queue.pool())
        .await
        .expect("clean test tenant");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations through 000021"]
async fn canonical_message_reaches_smtp_and_persists_remote_acceptance() {
    let queue = Queue::connect(QueueConfig {
        database_url: test_database_url(),
        ..QueueConfig::default()
    })
    .await
    .expect("connect test database");
    clean(&queue).await;

    let canonical = CanonicalMessage::parse(
        MessageEnvelope {
            sender: "sender@example.test".into(),
            recipients: vec!["recipient@example.test".into()],
        },
        b"Subject: M03 to M05 integration\r\n\r\nmessage body".to_vec(),
        MimeLimits::default(),
    )
    .expect("parse canonical message");
    let enqueued = queue
        .enqueue(&EnqueueRequest::new(
            &canonical,
            "m03-m05-e2e".into(),
            "delivery-integration-test".into(),
        ))
        .await
        .expect("enqueue canonical message");
    let message_id = enqueued.message_id;
    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name, status, created_at, updated_at) VALUES ($1, 'Test Tenant', 'active', NOW(), NOW())")
        .bind(tenant_id)
        .execute(queue.pool())
        .await
        .expect("create test tenant");
    sqlx::query(
        "UPDATE mail_messages SET tenant_id = $1, sender_domain = 'example.test', delivery_mode = 'direct_mx', dkim_key_ref = 'secret-manager://dkim/example.test' WHERE id = $2",
    )
    .bind(tenant_id)
    .bind(message_id)
    .execute(queue.pool())
    .await
    .expect("update delivery metadata");

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind SMTP fixture");
    let endpoint = listener.local_addr().expect("get fixture address");
    let smtp_fixture = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept SMTP connection");
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        write_half.write_all(b"220 fixture\r\n").await.unwrap();

        let mut line = String::new();
        for expected in ["EHLO", "MAIL FROM", "RCPT TO"] {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert!(
                line.starts_with(expected),
                "unexpected SMTP command: {line}"
            );
            write_half.write_all(b"250 ok\r\n").await.unwrap();
        }

        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "DATA\r\n");
        write_half.write_all(b"354 continue\r\n").await.unwrap();
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if line == ".\r\n" {
                break;
            }
            assert!(!line.is_empty(), "SMTP connection closed during DATA");
        }
        write_half.write_all(b"250 accepted\r\n").await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "QUIT\r\n");
        write_half.write_all(b"221 bye\r\n").await.unwrap();
    });

    let processor = Arc::new(SmtpProcessor {
        config: DeliveryConfig {
            enabled: true,
            test_endpoint: Some(endpoint),
            allow_loopback_test_destinations: true,
            allowed_recipient_domains: vec!["example.test".into()],
            command_timeout: Duration::from_secs(2),
            overall_timeout: Duration::from_secs(5),
            ..DeliveryConfig::default()
        },
        resolver: Arc::new(DnsResolver::default()),
    });
    let worker = DeliveryWorker {
        queue: queue.clone(),
        processor,
        owner: "m03-m05-integration".into(),
        poll_interval: Duration::from_millis(10),
    };
    let (shutdown, cancel) = tokio::sync::watch::channel(false);
    let worker_task = tokio::spawn(async move { worker.run_until_cancelled(cancel).await });

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if queue.state(message_id).await.expect("read queue state")
                == Some(QueueState::AcceptedByRemote)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("worker did not persist remote acceptance");

    shutdown.send(true).expect("signal worker shutdown");
    worker_task
        .await
        .expect("join delivery worker")
        .expect("delivery worker exits cleanly");
    smtp_fixture.await.expect("SMTP fixture succeeds");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL, migrations through 000021, and local Mailpit on 127.0.0.1:1025"]
async fn canonical_message_reaches_mailpit_and_persists_remote_acceptance() {
    let queue = Queue::connect(QueueConfig {
        database_url: test_database_url(),
        ..QueueConfig::default()
    })
    .await
    .expect("connect test database");
    clean(&queue).await;

    let canonical = CanonicalMessage::parse(
        MessageEnvelope {
            sender: "smtp-test@example.test".into(),
            recipients: vec!["recipient@example.test".into()],
        },
        b"From: smtp-test@example.test\r\nTo: recipient@example.test\r\nSubject: Mailpit worker integration\r\n\r\nCaptured by local Mailpit.".to_vec(),
        MimeLimits::default(),
    )
    .expect("parse canonical message");
    let enqueued = queue
        .enqueue(&EnqueueRequest::new(
            &canonical,
            "mailpit-worker-integration".into(),
            "mailpit-delivery-integration-test".into(),
        ))
        .await
        .expect("enqueue canonical message");
    let message_id = enqueued.message_id;
    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name, status, created_at, updated_at) VALUES ($1, 'Test Tenant', 'active', NOW(), NOW())")
        .bind(tenant_id)
        .execute(queue.pool())
        .await
        .expect("create test tenant");
    sqlx::query(
        "UPDATE mail_messages SET tenant_id = $1, sender_domain = 'example.test', delivery_mode = 'mailpit', dkim_key_ref = 'secret-manager://dkim/example.test' WHERE id = $2",
    )
    .bind(tenant_id)
    .bind(message_id)
    .execute(queue.pool())
    .await
    .expect("update delivery metadata");

    let processor = Arc::new(SmtpProcessor {
        config: DeliveryConfig {
            enabled: true,
            smtp_relay: Some(SmtpRelayConfig {
                host: "127.0.0.1".into(),
                port: 1025,
                username: String::new(),
                password: String::new(),
                from_address: "smtp-test@example.test".into(),
                mode: SmtpRelayMode::LocalMailpit,
            }),
            command_timeout: Duration::from_secs(2),
            overall_timeout: Duration::from_secs(5),
            ..DeliveryConfig::default()
        },
        resolver: Arc::new(DnsResolver::default()),
    });
    let worker = DeliveryWorker {
        queue: queue.clone(),
        processor,
        owner: "mailpit-worker-integration".into(),
        poll_interval: Duration::from_millis(10),
    };
    let (shutdown, cancel) = tokio::sync::watch::channel(false);
    let worker_task = tokio::spawn(async move { worker.run_until_cancelled(cancel).await });

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if queue.state(message_id).await.expect("read queue state")
                == Some(QueueState::AcceptedByRemote)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("worker did not persist Mailpit acceptance");

    shutdown.send(true).expect("signal worker shutdown");
    worker_task
        .await
        .expect("join delivery worker")
        .expect("delivery worker exits cleanly");
}
