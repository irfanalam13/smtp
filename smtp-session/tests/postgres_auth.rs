use chrono::{Duration, Utc};
use shared_types::{CanonicalMessage, MessageEnvelope, MimeLimits};
use smtp_ingress::{
    Authenticator, Config, PostgresAuthenticator, Server,
    inbound::{InboundAcceptor, InboundContext, PostgresInboundStore},
};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::{net::IpAddr, sync::Arc, time::Duration as StdDuration};
use tokio::time::timeout;
use uuid::Uuid;

fn test_database_url() -> String {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("set TEST_DATABASE_URL to an isolated disposable PostgreSQL database");
    assert!(
        url.contains("test"),
        "TEST_DATABASE_URL must identify an isolated test database"
    );
    url
}

fn inbound_context() -> InboundContext {
    InboundContext {
        peer_ip: "192.0.2.44".parse().unwrap(),
        helo_domain: "sender.example".into(),
        received_by: "smtp-test".into(),
        received_over_tls: true,
    }
}

async fn create_tenant(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(format!("smtp-auth-test-{id}"))
        .execute(pool)
        .await
        .expect("insert test tenant");
    id
}

async fn delete_tenant(pool: &PgPool, tenant_id: Uuid) {
    sqlx::query("DELETE FROM smtp_credentials WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("delete test credentials");
    sqlx::query("DELETE FROM domains WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("delete test domains");
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("delete test tenant");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn inbound_store_persists_versioned_metadata_and_exact_raw_message() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let tenant_id = create_tenant(&pool).await;
    let domain_id = Uuid::new_v4();
    let mailbox_id = Uuid::new_v4();
    let second_mailbox_id = Uuid::new_v4();
    let alias_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domains (id, tenant_id, domain, status)
         VALUES ($1, $2, 'canonical.example.test', 'verified')",
    )
    .bind(domain_id)
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("create verified test domain");
    sqlx::query(
        "INSERT INTO mailboxes (id, tenant_id, domain_id, local_part)
         VALUES ($1, $2, $3, 'recipient')",
    )
    .bind(mailbox_id)
    .bind(tenant_id)
    .bind(domain_id)
    .execute(&pool)
    .await
    .expect("create test mailbox");
    sqlx::query(
        "INSERT INTO mailboxes (id, tenant_id, domain_id, local_part)
         VALUES ($1, $2, $3, 'second')",
    )
    .bind(second_mailbox_id)
    .bind(tenant_id)
    .bind(domain_id)
    .execute(&pool)
    .await
    .expect("create second same-tenant test mailbox");
    sqlx::query(
        "INSERT INTO mailbox_aliases (id, tenant_id, mailbox_id, domain_id, local_part)
         VALUES ($1, $2, $3, $4, 'second-alias')",
    )
    .bind(alias_id)
    .bind(tenant_id)
    .bind(second_mailbox_id)
    .bind(domain_id)
    .execute(&pool)
    .await
    .expect("create alias for second mailbox");

    let raw = b"From: header-sender@example.test\r\n\
To: header-recipient@example.test\r\n\
Subject: inbound metadata\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
preserve these exact bytes\r\n"
        .to_vec();
    let canonical = CanonicalMessage::parse(
        MessageEnvelope {
            sender: "bounce@example.test".into(),
            recipients: vec![
                "recipient@canonical.example.test".into(),
                "second@canonical.example.test".into(),
                "second-alias@canonical.example.test".into(),
            ],
        },
        raw.clone(),
        MimeLimits::default(),
    )
    .expect("parse canonical inbound message");
    let inbound = PostgresInboundStore::from_pool(pool.clone(), usize::MAX);
    let context = inbound_context();
    let accepted = inbound
        .accept(&canonical, &context)
        .await
        .expect("persist canonical inbound message");
    assert!(!accepted.duplicate);
    assert!(
        inbound
            .accept(&canonical, &context)
            .await
            .expect("deduplicate inbound message")
            .duplicate
    );
    let stored = sqlx::query(
        "SELECT tenant_id, envelope_sender, raw_message, message_schema_version,
                parsed_metadata, subject, received_from_ip::text AS received_from_ip,
                received_by, received_over_tls
         FROM inbound_messages WHERE id = $1",
    )
    .bind(accepted.message_id)
    .fetch_one(&pool)
    .await
    .expect("read persisted canonical message");
    assert_eq!(stored.try_get::<Uuid, _>("tenant_id").unwrap(), tenant_id);
    assert_eq!(
        stored.try_get::<String, _>("envelope_sender").unwrap(),
        "bounce@example.test"
    );
    assert_eq!(stored.try_get::<Vec<u8>, _>("raw_message").unwrap(), raw);
    assert_eq!(
        stored.try_get::<i16, _>("message_schema_version").unwrap(),
        1
    );
    let metadata: serde_json::Value = stored.try_get("parsed_metadata").unwrap();
    assert_eq!(metadata["subject"], "inbound metadata");
    assert_eq!(metadata["from"][0], "header-sender@example.test");
    assert_eq!(
        stored.try_get::<String, _>("subject").unwrap(),
        "inbound metadata"
    );
    assert_eq!(
        stored.try_get::<String, _>("received_from_ip").unwrap(),
        "192.0.2.44"
    );
    assert_eq!(
        stored.try_get::<String, _>("received_by").unwrap(),
        "smtp-test"
    );
    assert!(stored.try_get::<bool, _>("received_over_tls").unwrap());
    let fanout_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inbound_messages WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&pool)
            .await
            .expect("count fan-out message copies");
    assert_eq!(fanout_count, 2, "each mailbox receives one durable copy");
    let recipient_link_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inbound_recipients
         WHERE message_id IN (SELECT id FROM inbound_messages WHERE tenant_id = $1)",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await
    .expect("count primary and alias recipients");
    assert_eq!(recipient_link_count, 3);
    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mailbox_event_outbox
         WHERE tenant_id = $1 AND event_type = 'inbound.message.accepted'",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await
    .expect("count durable inbound outbox events");
    assert_eq!(outbox_count, 2);

    sqlx::query("UPDATE mailboxes SET quota_bytes = 1, used_bytes = 0 WHERE id = $1")
        .bind(second_mailbox_id)
        .execute(&pool)
        .await
        .expect("lower test mailbox quota");
    let over_quota = CanonicalMessage::parse(
        MessageEnvelope {
            sender: "bounce@example.test".into(),
            recipients: vec!["second@canonical.example.test".into()],
        },
        raw.clone(),
        MimeLimits::default(),
    )
    .expect("parse over-quota message");
    assert!(matches!(
        inbound.accept(&over_quota, &context).await,
        Err(smtp_ingress::inbound::InboundError::QuotaExceeded)
    ));
    let rolled_back_quota: i64 =
        sqlx::query_scalar("SELECT used_bytes FROM mailboxes WHERE id = $1")
            .bind(second_mailbox_id)
            .fetch_one(&pool)
            .await
            .expect("read quota after rejected transaction");
    assert_eq!(rolled_back_quota, 0);

    let limiter_key = Uuid::new_v4().as_bytes().to_vec();
    let limiter_key = [limiter_key.as_slice(), b"extra entropy for inbound limiter"].concat();
    let limited_inbound = PostgresInboundStore::from_pool(pool.clone(), usize::MAX)
        .with_rate_limiter(&limiter_key, 1)
        .expect("configure shared inbound limiter");
    for (body, expected) in [
        (b"rate one".as_slice(), true),
        (b"rate two".as_slice(), false),
    ] {
        let rate_limited_message = CanonicalMessage::parse(
            MessageEnvelope {
                sender: "bounce@example.test".into(),
                recipients: vec!["recipient@canonical.example.test".into()],
            },
            [b"Subject: rate test\r\n\r\n".as_slice(), body].concat(),
            MimeLimits::default(),
        )
        .expect("parse rate-limited message");
        let result = limited_inbound
            .accept(&rate_limited_message, &context)
            .await;
        if expected {
            assert!(result.is_ok(), "first domain message is accepted");
        } else {
            assert!(matches!(
                result,
                Err(smtp_ingress::inbound::InboundError::RateLimited)
            ));
        }
    }
    let peer_ip: IpAddr = "198.51.100.10".parse().unwrap();
    let rate_limited_recipient = ["recipient@canonical.example.test".to_string()];
    limited_inbound
        .reserve_rate_capacity(peer_ip, &rate_limited_recipient)
        .await
        .expect("first message is inside rate limit");
    assert!(matches!(
        limited_inbound
            .reserve_rate_capacity(peer_ip, &rate_limited_recipient)
            .await,
        Err(smtp_ingress::inbound::InboundError::RateLimited)
    ));

    let other_tenant = create_tenant(&pool).await;
    let other_domain_id = Uuid::new_v4();
    let other_mailbox_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domains (id, tenant_id, domain, status)
         VALUES ($1, $2, 'other-canonical.example.test', 'verified')",
    )
    .bind(other_domain_id)
    .bind(other_tenant)
    .execute(&pool)
    .await
    .expect("create second tenant verified domain");
    sqlx::query(
        "INSERT INTO mailboxes (id, tenant_id, domain_id, local_part)
         VALUES ($1, $2, $3, 'recipient')",
    )
    .bind(other_mailbox_id)
    .bind(other_tenant)
    .bind(other_domain_id)
    .execute(&pool)
    .await
    .expect("create second tenant mailbox");
    let cross_tenant = CanonicalMessage::parse(
        MessageEnvelope {
            sender: "bounce@example.test".into(),
            recipients: vec![
                "recipient@canonical.example.test".into(),
                "recipient@other-canonical.example.test".into(),
            ],
        },
        raw.clone(),
        MimeLimits::default(),
    )
    .expect("parse cross-tenant message");
    assert!(matches!(
        inbound.accept(&cross_tenant, &context).await,
        Err(smtp_ingress::inbound::InboundError::Rejected)
    ));
    let cross_tenant_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inbound_messages WHERE tenant_id = ANY($1)")
            .bind(vec![tenant_id, other_tenant])
            .fetch_one(&pool)
            .await
            .expect("verify cross-tenant transaction left no rows");
    assert_eq!(cross_tenant_rows, 3, "cross-tenant rejection is atomic");

    sqlx::query(
        "DELETE FROM inbound_recipients WHERE message_id IN
         (SELECT id FROM inbound_messages WHERE tenant_id = ANY($1))",
    )
    .bind(vec![tenant_id, other_tenant])
    .execute(&pool)
    .await
    .expect("delete inbound recipient fixture");
    sqlx::query("DELETE FROM inbound_messages WHERE tenant_id = ANY($1)")
        .bind(vec![tenant_id, other_tenant])
        .execute(&pool)
        .await
        .expect("delete inbound message fixture");
    sqlx::query("DELETE FROM mailboxes WHERE id = ANY($1)")
        .bind(vec![mailbox_id, second_mailbox_id, other_mailbox_id])
        .execute(&pool)
        .await
        .expect("delete mailbox fixture");
    sqlx::query("DELETE FROM mailbox_aliases WHERE id = $1")
        .bind(alias_id)
        .execute(&pool)
        .await
        .expect("delete mailbox alias fixture");
    sqlx::query("DELETE FROM domains WHERE id = ANY($1)")
        .bind(vec![domain_id, other_domain_id])
        .execute(&pool)
        .await
        .expect("delete domain fixture");
    delete_tenant(&pool, tenant_id).await;
    delete_tenant(&pool, other_tenant).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn credentials_are_tenant_scoped_hashed_rotatable_and_revocable() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let auth = PostgresAuthenticator::from_pool(pool.clone());
    let first_tenant = create_tenant(&pool).await;
    let second_tenant = create_tenant(&pool).await;
    let username = "mailer@example.test";
    let first_password = "first long password";
    let rotated_password = "rotated long password";

    auth.provision(first_tenant, username, first_password)
        .await
        .expect("provision first credential");
    auth.provision(second_tenant, username, "another long password")
        .await
        .expect("provision same username for another tenant");
    sqlx::query(
        "INSERT INTO domains (id, tenant_id, domain, status)
         VALUES ($1, $2, 'example.test', 'verified')",
    )
    .bind(Uuid::new_v4())
    .bind(first_tenant)
    .execute(&pool)
    .await
    .expect("create verified test domain");
    sqlx::query("UPDATE tenants SET max_smtp_message_bytes = 2048 WHERE id = $1")
        .bind(first_tenant)
        .execute(&pool)
        .await
        .expect("set tenant message-size limit");
    sqlx::query(
        "UPDATE domains SET max_smtp_message_bytes = 1024
         WHERE tenant_id = $1 AND domain = 'example.test'",
    )
    .bind(first_tenant)
    .execute(&pool)
    .await
    .expect("set domain message-size limit");
    assert_eq!(
        auth.max_message_bytes(first_tenant, "mailer@example.test")
            .await
            .unwrap(),
        1024
    );
    assert_eq!(
        auth.max_message_bytes(first_tenant, "mailer@other.example")
            .await
            .unwrap(),
        2048
    );
    assert_eq!(
        auth.max_message_bytes(first_tenant, "").await.unwrap(),
        2048
    );
    let inbound = PostgresInboundStore::from_pool(pool.clone(), usize::MAX);
    assert_eq!(
        inbound
            .max_message_bytes(&["recipient@example.test".into()])
            .await
            .unwrap(),
        Some(1024)
    );
    assert!(
        auth.authorize_sender(first_tenant, "mailer@example.test")
            .await
            .unwrap()
    );
    assert!(
        !auth
            .authorize_sender(second_tenant, "mailer@example.test")
            .await
            .unwrap()
    );
    assert!(
        !auth
            .authorize_sender(first_tenant, "mailer@unverified.test")
            .await
            .unwrap()
    );

    let credential = sqlx::query(
        "SELECT password_hash, password_hash_version
         FROM smtp_credentials WHERE tenant_id = $1 AND username = $2",
    )
    .bind(first_tenant)
    .bind(username)
    .fetch_one(&pool)
    .await
    .expect("read test credential");
    let password_hash: String = credential.try_get("password_hash").unwrap();
    let hash_version: i16 = credential.try_get("password_hash_version").unwrap();
    assert!(password_hash.starts_with("$argon2id$"));
    assert_ne!(password_hash, first_password);
    assert_eq!(hash_version, 1);
    assert!(
        auth.authenticate(&format!("{first_tenant}:{username}"), first_password)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        auth.authenticate(&format!("{first_tenant}:{username}"), "incorrect password")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        auth.authenticate(&format!("{second_tenant}:{username}"), first_password)
            .await
            .unwrap()
            .is_none()
    );

    auth.provision(first_tenant, username, rotated_password)
        .await
        .expect("rotate test credential");
    assert!(
        auth.authenticate(&format!("{first_tenant}:{username}"), first_password)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        auth.authenticate(&format!("{first_tenant}:{username}"), rotated_password)
            .await
            .unwrap()
            .is_some()
    );
    assert!(auth.revoke(first_tenant, username).await.unwrap());
    assert!(
        auth.authenticate(&format!("{first_tenant}:{username}"), rotated_password)
            .await
            .unwrap()
            .is_none()
    );

    auth.provision_with_expiry(
        second_tenant,
        username,
        "another long password",
        Some(Utc::now() - Duration::minutes(1)),
    )
    .await
    .expect("provision expired credential");
    assert!(
        auth.authenticate(
            &format!("{second_tenant}:{username}"),
            "another long password"
        )
        .await
        .unwrap()
        .is_none()
    );

    delete_tenant(&pool, first_tenant).await;
    delete_tenant(&pool, second_tenant).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn smtp_auth_rate_limits_are_shared_across_ingress_instances() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let first = PostgresAuthenticator::from_pool(pool.clone())
        .shared_rate_limiter(&[0x41; 32])
        .expect("configure first shared limiter");
    let second = PostgresAuthenticator::from_pool(pool.clone())
        .shared_rate_limiter(&[0x41; 32])
        .expect("configure second shared limiter");
    let tenant_id = Uuid::new_v4();
    let authcid = format!("{tenant_id}:shared-limit-test-{}", Uuid::new_v4());
    let peer = IpAddr::V6(std::net::Ipv6Addr::from(*tenant_id.as_bytes()));

    for _ in 0..5 {
        first
            .record_failure(peer, &authcid)
            .await
            .expect("record shared failed authentication");
    }
    assert!(
        second
            .is_limited(peer, &authcid)
            .await
            .expect("check shared account lock")
    );

    second
        .record_success(peer, &authcid)
        .await
        .expect("clear account and tenant lock on success");
    assert!(
        !first
            .is_limited(peer, &authcid)
            .await
            .expect("recheck cleared account lock")
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL, Python 3, and migrations"]
async fn python_smtplib_completes_starttls_authenticated_submission() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let auth = Arc::new(PostgresAuthenticator::from_pool(pool.clone()));
    let tenant_id = create_tenant(&pool).await;
    let username = format!("interop-{}", Uuid::new_v4());
    let password = "external-client-test-password";
    let sender_domain = format!("interop-{}.example.test", Uuid::new_v4());
    auth.provision(tenant_id, &username, password)
        .await
        .expect("provision SMTP test credential");
    sqlx::query(
        "INSERT INTO domains (id, tenant_id, domain, status)
         VALUES ($1, $2, $3, 'verified')",
    )
    .bind(Uuid::new_v4())
    .bind(tenant_id)
    .bind(&sender_domain)
    .execute(&pool)
    .await
    .expect("insert verified test sender domain");
    sqlx::query("UPDATE tenants SET max_smtp_message_bytes = 2048 WHERE id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("set tenant submission size limit");
    sqlx::query(
        "UPDATE domains SET max_smtp_message_bytes = 1024
         WHERE tenant_id = $1 AND domain = $2",
    )
    .bind(tenant_id)
    .bind(&sender_domain)
    .execute(&pool)
    .await
    .expect("set sender-domain submission size limit");

    let _ = rustls::crypto::ring::default_provider().install_default();
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
        .expect("generate test certificate");
    let private_key =
        rustls::pki_types::PrivateKeyDer::Pkcs8(certificate.key_pair.serialize_der().into());
    let tls = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate.cert.der().clone()], private_key)
        .expect("build test TLS configuration");
    let rate_limiter = Arc::new(
        auth.shared_rate_limiter(&[0x5a; 32])
            .expect("configure shared auth throttling"),
    );
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        tls: Some(Arc::new(tls)),
        authenticator: Some(auth.clone()),
        sender_authorizer: Some(auth.clone()),
        shared_auth_rate_limiter: Some(rate_limiter),
        require_authentication: true,
        ..Config::default()
    };
    let server = Server::bind(config).await.expect("bind SMTP server");
    let address = server.local_addr().expect("get SMTP server address");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_task = tokio::spawn(async move {
        server
            .run(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("serve SMTP test connection");
    });

    let authcid = format!("{tenant_id}:{username}");
    let output = timeout(
        StdDuration::from_secs(15),
        tokio::process::Command::new("python3")
            .arg("-c")
            .arg(
                "import smtplib, ssl, sys\n\
                 host, port, authcid, password, sender = sys.argv[1:]\n\
                 smtp = smtplib.SMTP(host, int(port), timeout=10)\n\
                 smtp.ehlo()\n\
                 smtp.starttls(context=ssl._create_unverified_context())\n\
                 smtp.ehlo()\n\
                 smtp.login(authcid, password)\n\
                 code, _ = smtp.mail(sender, options=['SIZE=1025'])\n\
                 assert code == 552, code\n\
                 refused = smtp.sendmail(sender, ['recipient@example.net'], 'Subject: external SMTP client test\\r\\n\\r\\ntest body')\n\
                 assert not refused, refused\n\
                 smtp.quit()\n",
            )
            .arg(address.ip().to_string())
            .arg(address.port().to_string())
            .arg(authcid)
            .arg(password)
            .arg(format!("sender@{sender_domain}"))
            .output(),
    )
        .await
        .expect("Python SMTP client deadline")
        .expect("run Python SMTP client");
    assert!(
        output.status.success(),
        "Python smtplib failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = shutdown_tx.send(());
    server_task.await.expect("SMTP server task");
    delete_tenant(&pool, tenant_id).await;
}
