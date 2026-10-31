use chrono::Utc;
use shared_types::{
    CreateMailbox, CreateMailboxLabel, CreateSentSubmission, MailRule, MailboxFlag, MailboxKind,
    MailboxPermission, MailboxStatus, MailboxStore, MailboxStoreError, QuarantineRequest,
    RuleAction, RuleCondition, RuleDirection, RuleSizeOperator, SentRecipientEvent, search_mailbox,
};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
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

async fn insert_tenant_fixture(pool: &PgPool, tenant_id: Uuid, domain_id: Uuid, user_id: Uuid) {
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, 'mailbox integration test')")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("insert tenant");
    sqlx::query("INSERT INTO users (id, external_subject) VALUES ($1, $2)")
        .bind(user_id)
        .bind(format!("mailbox-test-{user_id}"))
        .execute(pool)
        .await
        .expect("insert user");
    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, user_id, role)
         VALUES ($1, $2, 'owner')",
    )
    .bind(tenant_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert owner membership");
    sqlx::query(
        "INSERT INTO domains (id, tenant_id, domain, status)
         VALUES ($1, $2, $3, 'verified')",
    )
    .bind(domain_id)
    .bind(tenant_id)
    .bind(format!("{}.mailbox-test.invalid", domain_id.simple()))
    .execute(pool)
    .await
    .expect("insert verified domain");
}

async fn insert_inbound_message_fixture(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    mailbox_id: Uuid,
    idempotency_key: Vec<u8>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO inbound_messages
     (id, tenant_id, mailbox_id, envelope_sender, raw_message, raw_size, idempotency_key)
     VALUES ($1, $2, $3, 'sender@example.test', $4, 7, $5)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(mailbox_id)
    .bind(b"message".as_slice())
    .bind(idempotency_key)
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn inbound_message_cannot_reference_a_mailbox_in_another_tenant() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let tenant_a = Uuid::new_v4();
    let domain_a = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let domain_b = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    insert_tenant_fixture(&pool, tenant_a, domain_a, user_a).await;
    insert_tenant_fixture(&pool, tenant_b, domain_b, user_b).await;

    let mailbox_a = Uuid::new_v4();
    let mailbox_b = Uuid::new_v4();
    for (mailbox_id, tenant_id, domain_id, local_part) in [
        (mailbox_a, tenant_a, domain_a, "alpha"),
        (mailbox_b, tenant_b, domain_b, "bravo"),
    ] {
        sqlx::query(
            "INSERT INTO mailboxes (id, tenant_id, domain_id, local_part)
         VALUES ($1, $2, $3, $4)",
        )
        .bind(mailbox_id)
        .bind(tenant_id)
        .bind(domain_id)
        .bind(local_part)
        .execute(&pool)
        .await
        .expect("insert mailbox fixture");
    }

    insert_inbound_message_fixture(&pool, Uuid::new_v4(), tenant_a, mailbox_a, vec![1; 32])
        .await
        .expect("same-tenant inbound message is valid");
    let error =
        insert_inbound_message_fixture(&pool, Uuid::new_v4(), tenant_a, mailbox_b, vec![2; 32])
            .await
            .expect_err("cross-tenant mailbox reference must be rejected");
    assert!(matches!(
    error,
    sqlx::Error::Database(ref database_error)
        if database_error.code().as_deref() == Some("23503")
    ));
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn provisioning_quota_and_mailbox_lifecycle_are_transactional() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let tenant_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let collaborator_id = Uuid::new_v4();
    insert_tenant_fixture(&pool, tenant_id, domain_id, owner_id).await;
    sqlx::query("INSERT INTO users (id, external_subject) VALUES ($1, $2)")
        .bind(collaborator_id)
        .bind(format!("mailbox-test-{collaborator_id}"))
        .execute(&pool)
        .await
        .expect("insert collaborator");
    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, user_id, role)
         VALUES ($1, $2, 'member')",
    )
    .bind(tenant_id)
    .bind(collaborator_id)
    .execute(&pool)
    .await
    .expect("insert collaborator membership");

    let store = MailboxStore::from_pool(pool.clone(), 64);
    let mailbox = store
        .create_mailbox(&CreateMailbox {
            tenant_id,
            domain_id,
            local_part: "team".into(),
            display_name: "Team mailbox".into(),
            owner_user_id: Some(owner_id),
            kind: MailboxKind::Shared,
            quota_bytes: 10,
        })
        .await
        .expect("create mailbox with default folders");
    assert_eq!(mailbox.status, MailboxStatus::Active);

    let folders = store
        .ensure_default_folders(mailbox.id)
        .await
        .expect("repeat default provisioning");
    assert_eq!(folders.len(), 6);
    let inbox = folders
        .iter()
        .find(|folder| folder.name == "INBOX")
        .unwrap();
    assert!(matches!(
        store
            .append_for_user(
                tenant_id,
                collaborator_id,
                inbox.id,
                b"unauthorized",
                &[],
                Utc::now(),
            )
            .await,
        Err(MailboxStoreError::MailboxNotFound)
    ));
    let nested = store
        .create_child_folder(inbox.id, "Receipts")
        .await
        .expect("create nested folder");

    store
        .add_alias(tenant_id, mailbox.id, domain_id, "billing")
        .await
        .expect("create mailbox alias");
    assert!(
        store
            .add_alias(tenant_id, mailbox.id, domain_id, "TEAM")
            .await
            .is_err(),
        "an alias must not collide with a primary mailbox address"
    );
    store
        .set_permission(
            tenant_id,
            mailbox.id,
            collaborator_id,
            MailboxPermission::Read,
        )
        .await
        .expect("grant mailbox permission");
    assert!(
        store
            .revoke_permission(tenant_id, mailbox.id, collaborator_id)
            .await
            .expect("revoke mailbox permission")
    );

    let first = store
        .append(inbox.id, b"raw", &[], Utc::now())
        .await
        .expect("append message");
    assert_eq!(first.uid, 1);
    assert!(matches!(
        store.append(nested.id, b"too large", &[], Utc::now()).await,
        Err(MailboxStoreError::QuotaExceeded)
    ));
    assert!(store.mark_deleted(first.id).await.expect("mark deleted"));
    assert_eq!(store.expunge(inbox.id).await.expect("expunge"), 1);

    assert!(
        store
            .transition_mailbox(mailbox.id, MailboxStatus::Suspended)
            .await
            .expect("suspend mailbox")
    );
    assert!(matches!(
        store
            .append(inbox.id, b"blocked", &[MailboxFlag::Seen], Utc::now())
            .await,
        Err(MailboxStoreError::MailboxUnavailable)
    ));
    assert!(
        store
            .transition_mailbox(mailbox.id, MailboxStatus::Active)
            .await
            .expect("reactivate mailbox")
    );
    let rule_id = Uuid::new_v4();
    assert_eq!(
        store
            .save_rule(
                tenant_id,
                owner_id,
                mailbox.id,
                &MailRule {
                    id: rule_id,
                    direction: RuleDirection::Incoming,
                    enabled: true,
                    priority: 10,
                    stop_processing: false,
                    condition: RuleCondition::Size {
                        operator: RuleSizeOperator::GreaterThan,
                        bytes: 1,
                    },
                    actions: vec![RuleAction::MarkRead],
                },
            )
            .await
            .expect("persist incoming mailbox rule"),
        1
    );
    let second = store
        .append(inbox.id, b"new", &[], Utc::now())
        .await
        .expect("append after reactivation");
    assert_eq!(second.uid, 2);
    assert_eq!(second.flags, vec![MailboxFlag::Seen]);
    let quota = store
        .mailbox_quota(tenant_id, owner_id, mailbox.id)
        .await
        .expect("read mailbox quota");
    assert_eq!(quota.used_bytes, 3);
    assert_eq!(quota.quota_bytes, 10);
    assert_eq!(quota.warning, None);
    assert!(matches!(
        store
            .set_mailbox_quota(tenant_id, owner_id, mailbox.id, 2)
            .await,
        Err(MailboxStoreError::InvalidWorkflowState)
    ));
    assert_eq!(
        store
            .set_mailbox_quota(tenant_id, owner_id, mailbox.id, 5)
            .await
            .expect("increase mailbox quota")
            .quota_bytes,
        5
    );
    assert!(
        store
            .set_message_flags(
                tenant_id,
                owner_id,
                mailbox.id,
                second.id,
                &[MailboxFlag::Seen, MailboxFlag::Flagged],
            )
            .await
            .expect("set read/star flags through authorized operation")
    );
    sqlx::query("UPDATE mailbox_messages SET custom_properties = $1 WHERE id = $2")
        .bind(serde_json::json!({"client_annotation": "checked"}))
        .bind(second.id)
        .execute(&pool)
        .await
        .expect("update message custom properties");
    assert_eq!(
        store
            .list_rules(tenant_id, owner_id, mailbox.id)
            .await
            .expect("list mailbox rules")
            .len(),
        1
    );
    store
        .set_permission(
            tenant_id,
            mailbox.id,
            collaborator_id,
            MailboxPermission::Read,
        )
        .await
        .expect("grant read-only workflow access");
    assert!(matches!(
        store
            .set_message_flags(
                tenant_id,
                collaborator_id,
                mailbox.id,
                second.id,
                &[MailboxFlag::Seen],
            )
            .await,
        Err(MailboxStoreError::InvalidWorkflowState)
    ));
    assert!(matches!(
        store
            .create_label(
                tenant_id,
                collaborator_id,
                mailbox.id,
                &CreateMailboxLabel {
                    name: "Unauthorized".into(),
                    kind: "label".into(),
                    color: None,
                    query_definition: None,
                },
            )
            .await,
        Err(MailboxStoreError::InvalidWorkflowState)
    ));
    assert!(
        store
            .changes_since(tenant_id, collaborator_id, mailbox.id, 0, 10)
            .await
            .is_ok(),
        "read-only permission permits sync reads"
    );
    store
        .revoke_permission(tenant_id, mailbox.id, collaborator_id)
        .await
        .expect("revoke read-only workflow access");
    sqlx::query(
        "INSERT INTO mailbox_drafts
         (mailbox_id, tenant_id, message_id, owner_user_id, updated_by, current_version)
         VALUES ($1, $2, $3, $4, $4, 1)",
    )
    .bind(mailbox.id)
    .bind(tenant_id)
    .bind(second.id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("create mailbox draft metadata");
    sqlx::query(
        "INSERT INTO mailbox_draft_revisions
         (mailbox_id, tenant_id, message_id, version, raw_message, editor_user_id)
         VALUES ($1, $2, $3, 1, $4, $5)",
    )
    .bind(mailbox.id)
    .bind(tenant_id)
    .bind(second.id)
    .bind(b"draft-v1".as_slice())
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect_err("draft snapshots must participate in quota accounting");
    let used_after_rejected_revision: i64 =
        sqlx::query_scalar("SELECT used_bytes FROM mailboxes WHERE id = $1")
            .bind(mailbox.id)
            .fetch_one(&pool)
            .await
            .expect("read quota after rejected draft snapshot");
    assert_eq!(used_after_rejected_revision, 3);

    sqlx::query(
        "INSERT INTO mailbox_draft_revisions
         (mailbox_id, tenant_id, message_id, version, raw_message, editor_user_id)
         VALUES ($1, $2, $3, 1, $4, $5)",
    )
    .bind(mailbox.id)
    .bind(tenant_id)
    .bind(second.id)
    .bind(b"d".as_slice())
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("charge draft snapshot bytes");
    let used_after_revision: i64 =
        sqlx::query_scalar("SELECT used_bytes FROM mailboxes WHERE id = $1")
            .bind(mailbox.id)
            .fetch_one(&pool)
            .await
            .expect("read quota after draft snapshot");
    assert_eq!(used_after_revision, 4);
    assert!(
        sqlx::query(
            "UPDATE mailbox_draft_revisions SET raw_message = $1
             WHERE mailbox_id = $2 AND message_id = $3 AND version = 1",
        )
        .bind(b"replacement".as_slice())
        .bind(mailbox.id)
        .bind(second.id)
        .execute(&pool)
        .await
        .is_err(),
        "draft history must be immutable"
    );
    sqlx::query(
        "DELETE FROM mailbox_draft_revisions
         WHERE mailbox_id = $1 AND message_id = $2 AND version = 1",
    )
    .bind(mailbox.id)
    .bind(second.id)
    .execute(&pool)
    .await
    .expect("release revision quota on retention deletion");
    let used_after_revision_delete: i64 =
        sqlx::query_scalar("SELECT used_bytes FROM mailboxes WHERE id = $1")
            .bind(mailbox.id)
            .fetch_one(&pool)
            .await
            .expect("read quota after revision deletion");
    assert_eq!(used_after_revision_delete, 3);

    let category = store
        .create_label(
            tenant_id,
            owner_id,
            mailbox.id,
            &CreateMailboxLabel {
                name: "Receipts".into(),
                kind: "category".into(),
                color: Some("#AABBCC".into()),
                query_definition: None,
            },
        )
        .await
        .expect("create mailbox category");
    assert!(
        store
            .assign_label(tenant_id, owner_id, mailbox.id, second.id, category.id)
            .await
            .expect("assign category to a message")
    );
    assert!(
        sqlx::query(
            "UPDATE mailbox_labels
             SET label_kind = 'smart', query_definition = '{\"subject\":\"receipt\"}'::jsonb
             WHERE id = $1",
        )
        .bind(category.id)
        .execute(&pool)
        .await
        .is_err(),
        "labels with assignments cannot be converted into virtual labels"
    );
    let smart_label = store
        .create_label(
            tenant_id,
            owner_id,
            mailbox.id,
            &CreateMailboxLabel {
                name: "Recent receipts".into(),
                kind: "smart".into(),
                color: None,
                query_definition: Some(serde_json::json!({"subject": "receipt"})),
            },
        )
        .await
        .expect("create a query-backed virtual label");
    assert!(
        store
            .assign_label(tenant_id, owner_id, mailbox.id, second.id, smart_label.id)
            .await
            .is_err(),
        "smart labels must be derived rather than assigned"
    );
    sqlx::query(
        "UPDATE mailbox_messages
         SET subject = 'Quarterly Revenue', sender_address = 'finance@example.test',
             recipients = ARRAY['owner@example.test'], body_text = 'Revenue increased',
             attachment_names = ARRAY['report.pdf'],
             custom_properties = '{\"source\":\"integration-test\"}'::jsonb
         WHERE id = $1",
    )
    .bind(second.id)
    .execute(&pool)
    .await
    .expect("populate MIME-derived search projection");
    let results = search_mailbox(
        &pool,
        tenant_id,
        mailbox.id,
        "\"Quarterly Revenue\"",
        false,
        10,
        0,
    )
    .await
    .expect("search this tenant mailbox");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].message_id, second.id);
    assert_eq!(results[0].uid, 2);
    assert!(
        search_mailbox(&pool, Uuid::new_v4(), mailbox.id, "Revenue", false, 10, 0)
            .await
            .expect("cross-tenant search returns no data")
            .is_empty()
    );
    sqlx::query("UPDATE mailboxes SET quota_bytes = 8192 WHERE id = $1")
        .bind(mailbox.id)
        .execute(&pool)
        .await
        .expect("raise integration fixture quota");
    let mime_store = MailboxStore::from_pool(pool.clone(), 4096);
    let root_message = mime_store
        .append(
            inbox.id,
            b"From: sender@example.test\r\nTo: owner@example.test\r\nSubject: Thread root\r\nMessage-ID: <root@example.test>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nsearchable orchard body",
            &[],
            Utc::now(),
        )
        .await
        .expect("append parsed root message");
    let reply_message = mime_store
        .append(
            inbox.id,
            b"From: sender@example.test\r\nTo: owner@example.test\r\nSubject: Re: Thread root\r\nMessage-ID: <reply@example.test>\r\nIn-Reply-To: <root@example.test>\r\nReferences: <root@example.test>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nreply text",
            &[],
            Utc::now(),
        )
        .await
        .expect("append parsed reply");
    let root_projection = sqlx::query(
        "SELECT header_message_id, conversation_id, body_text
         FROM mailbox_messages WHERE id = $1",
    )
    .bind(root_message.id)
    .fetch_one(&pool)
    .await
    .expect("read root MIME projection");
    let reply_projection = sqlx::query(
        "SELECT header_message_id, conversation_id
         FROM mailbox_messages WHERE id = $1",
    )
    .bind(reply_message.id)
    .fetch_one(&pool)
    .await
    .expect("read reply MIME projection");
    let root_conversation: Uuid = root_projection
        .try_get("conversation_id")
        .expect("root conversation ID");
    assert_eq!(
        reply_projection
            .try_get::<Option<String>, _>("header_message_id")
            .expect("reply Message-ID")
            .as_deref(),
        Some("reply@example.test")
    );
    assert_eq!(
        reply_projection
            .try_get::<Uuid, _>("conversation_id")
            .expect("reply conversation ID"),
        root_conversation
    );
    assert!(
        root_projection
            .try_get::<String, _>("body_text")
            .expect("root body projection")
            .contains("searchable orchard")
    );
    assert!(
        search_mailbox(&pool, tenant_id, mailbox.id, "orchard", false, 10, 0)
            .await
            .expect("search MIME-populated projection")
            .iter()
            .any(|hit| hit.message_id == root_message.id)
    );
    let junk_folder = folders
        .iter()
        .find(|folder| folder.special_use.as_deref() == Some("\\Junk"))
        .expect("default Junk folder");
    assert!(
        store
            .quarantine_message(
                tenant_id,
                owner_id,
                mailbox.id,
                &QuarantineRequest {
                    message_id: root_message.id,
                    junk_folder_id: junk_folder.id,
                    reason_code: "test_classifier".into(),
                    review_until: Some(Utc::now() + chrono::Duration::days(7)),
                },
            )
            .await
            .expect("quarantine a message")
    );
    assert!(
        store
            .release_quarantined_message(tenant_id, owner_id, mailbox.id, root_message.id)
            .await
            .expect("release a reviewed quarantine item")
    );

    let draft_folder = folders
        .iter()
        .find(|folder| folder.special_use.as_deref() == Some("\\Drafts"))
        .expect("default Drafts folder");
    let draft_message = store
        .append(draft_folder.id, b"d", &[], Utc::now())
        .await
        .expect("append a draft message");
    assert_eq!(
        store
            .save_draft_revision(tenant_id, owner_id, mailbox.id, draft_message.id, 0, b"x",)
            .await
            .expect("save the first draft revision"),
        1
    );
    assert!(matches!(
        store
            .save_draft_revision(
                tenant_id,
                owner_id,
                mailbox.id,
                draft_message.id,
                0,
                b"stale",
            )
            .await,
        Err(MailboxStoreError::VersionConflict)
    ));
    assert_eq!(
        store
            .prune_draft_revisions(tenant_id, owner_id, mailbox.id, draft_message.id, 1)
            .await
            .expect("keep the latest draft revision"),
        0
    );

    let submission_id = Uuid::new_v4();
    let sent_id = store
        .create_sent_submission(
            tenant_id,
            owner_id,
            mailbox.id,
            &CreateSentSubmission {
                message_id: second.id,
                submission_id,
                idempotency_key: [7; 32],
                queue_id: Some("queue-test-1".into()),
            },
        )
        .await
        .expect("record idempotent submission");
    let accepted_event = SentRecipientEvent {
        submission_id: sent_id,
        source_event_id: Uuid::new_v4(),
        recipient_address: "recipient@example.test".into(),
        state: "accepted".into(),
        status_code: Some("250 2.0.0".into()),
        diagnostic: None,
        occurred_at: Utc::now(),
    };
    assert!(
        store
            .record_sent_recipient_event(tenant_id, owner_id, mailbox.id, &accepted_event)
            .await
            .expect("consume accepted delivery event")
    );
    assert!(
        !store
            .record_sent_recipient_event(tenant_id, owner_id, mailbox.id, &accepted_event)
            .await
            .expect("duplicate queue event is idempotent")
    );
    let mut delivered_event = accepted_event.clone();
    delivered_event.source_event_id = Uuid::new_v4();
    delivered_event.state = "delivered".into();
    delivered_event.status_code = Some("250 2.0.0 delivered".into());
    delivered_event.occurred_at = Utc::now();
    assert!(
        store
            .record_sent_recipient_event(tenant_id, owner_id, mailbox.id, &delivered_event)
            .await
            .expect("consume final delivery event")
    );
    let submission_state: String =
        sqlx::query_scalar("SELECT submission_state FROM mailbox_sent_submissions WHERE id = $1")
            .bind(sent_id)
            .fetch_one(&pool)
            .await
            .expect("read sent submission state");
    assert_eq!(submission_state, "completed");

    store
        .set_archive_policy(tenant_id, owner_id, mailbox.id, second.id, None, true)
        .await
        .expect("place message under legal hold");
    let trash_folder = folders
        .iter()
        .find(|folder| folder.special_use.as_deref() == Some("\\Trash"))
        .expect("default Trash folder");
    assert!(
        store
            .move_to_trash(
                tenant_id,
                owner_id,
                mailbox.id,
                second.id,
                trash_folder.id,
                Some(Utc::now() - chrono::Duration::seconds(1)),
            )
            .await
            .expect("move message to recoverable Trash")
    );
    assert_eq!(
        store
            .purge_expired_trash(Utc::now(), 10)
            .await
            .expect("purge skips legal hold"),
        0
    );
    store
        .set_archive_policy(tenant_id, owner_id, mailbox.id, second.id, None, false)
        .await
        .expect("release legal hold");
    assert!(
        store
            .restore_from_trash(tenant_id, owner_id, mailbox.id, second.id)
            .await
            .expect("restore message with a new UID")
    );
    assert_eq!(
        store
            .purge_expired_trash(Utc::now(), 10)
            .await
            .expect("no longer in Trash"),
        0
    );
    store
        .move_to_trash(
            tenant_id,
            owner_id,
            mailbox.id,
            second.id,
            trash_folder.id,
            Some(Utc::now() - chrono::Duration::seconds(1)),
        )
        .await
        .expect("move restored mail back to Trash");
    assert_eq!(
        store
            .purge_expired_trash(Utc::now(), 10)
            .await
            .expect("purge expired unheld mail"),
        1
    );

    let changes = store
        .changes_since(tenant_id, owner_id, mailbox.id, 0, 100)
        .await
        .expect("read durable mailbox synchronization changes");
    assert!(changes.len() >= 5);
    assert!(changes.iter().any(|change| {
        change
            .payload
            .get("metadata_kind")
            .and_then(|kind| kind.as_str())
            == Some("label")
    }));
    assert!(changes.iter().any(|change| {
        change
            .payload
            .get("metadata_kind")
            .and_then(|kind| kind.as_str())
            == Some("draft")
    }));
    assert!(changes.iter().any(|change| {
        change
            .payload
            .get("metadata_kind")
            .and_then(|kind| kind.as_str())
            == Some("custom_properties")
    }));
    let worker_id = Uuid::new_v4();
    let outbox = store
        .claim_outbox_events(worker_id, 100, 30)
        .await
        .expect("claim mailbox event outbox");
    assert!(!outbox.is_empty());
    assert!(
        store
            .acknowledge_outbox_event(worker_id, outbox[0].id)
            .await
            .expect("acknowledge outbox delivery")
    );

    sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(mailbox.id)
        .execute(&pool)
        .await
        .expect("remove test mailbox");
    sqlx::query("DELETE FROM domains WHERE id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .expect("remove test domain");
    sqlx::query("DELETE FROM tenant_memberships WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("remove test memberships");
    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![owner_id, collaborator_id])
        .execute(&pool)
        .await
        .expect("remove test users");
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("remove test tenant");
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn idempotent_append_replays_one_message_and_rejects_key_reuse() {
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let tenant_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    insert_tenant_fixture(&pool, tenant_id, domain_id, owner_id).await;
    let store = MailboxStore::from_pool(pool.clone(), 1024);
    let mailbox = store
        .create_mailbox(&CreateMailbox {
            tenant_id,
            domain_id,
            local_part: format!("append-{}", Uuid::new_v4().simple()),
            display_name: "Idempotent append test".into(),
            owner_user_id: Some(owner_id),
            kind: MailboxKind::User,
            quota_bytes: 1024,
        })
        .await
        .expect("create mailbox");
    let inbox = store
        .ensure_default_folders(mailbox.id)
        .await
        .expect("create default folders")
        .into_iter()
        .find(|folder| folder.name == "INBOX")
        .expect("find INBOX");
    let raw = b"Subject: idempotent\r\n\r\nbody";
    let first = store
        .append_idempotent_for_user(
            (tenant_id, owner_id),
            inbox.id,
            raw,
            &[],
            Utc::now(),
            "append-test-001",
        )
        .await
        .expect("append message");
    let replay = store
        .append_idempotent_for_user(
            (tenant_id, owner_id),
            inbox.id,
            raw,
            &[],
            Utc::now(),
            "append-test-001",
        )
        .await
        .expect("replay original append");
    assert_eq!(replay.id, first.id);
    assert_eq!(replay.uid, first.uid);
    assert!(matches!(
        store
            .append_idempotent_for_user(
                (tenant_id, owner_id),
                inbox.id,
                b"Subject: changed\r\n\r\nbody",
                &[],
                Utc::now(),
                "append-test-001",
            )
            .await,
        Err(MailboxStoreError::IdempotencyConflict)
    ));
    let (message_count, used_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::BIGINT, mailbox.used_bytes
         FROM mailbox_messages AS message
         JOIN mailboxes AS mailbox ON mailbox.id = message.mailbox_id
         WHERE message.mailbox_id = $1
         GROUP BY mailbox.used_bytes",
    )
    .bind(mailbox.id)
    .fetch_one(&pool)
    .await
    .expect("read idempotent append accounting");
    assert_eq!(message_count, 1);
    assert_eq!(used_bytes, raw.len() as i64);

    sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(mailbox.id)
        .execute(&pool)
        .await
        .expect("remove test mailbox");
    sqlx::query("DELETE FROM domains WHERE id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .expect("remove test domain");
    sqlx::query("DELETE FROM tenant_memberships WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("remove test membership");
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("remove test user");
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("remove test tenant");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and migrations"]
async fn concurrent_appends_never_overreserve_mailbox_quota() {
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&test_database_url())
        .await
        .expect("connect isolated test database");
    let tenant_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    insert_tenant_fixture(&pool, tenant_id, domain_id, owner_id).await;
    let store = MailboxStore::from_pool(pool.clone(), 1024);
    let mailbox = store
        .create_mailbox(&CreateMailbox {
            tenant_id,
            domain_id,
            local_part: format!("quota-{}", Uuid::new_v4().simple()),
            display_name: "Concurrent quota test".into(),
            owner_user_id: Some(owner_id),
            kind: MailboxKind::User,
            quota_bytes: 7,
        })
        .await
        .expect("create quota-limited mailbox");
    let inbox = store
        .ensure_default_folders(mailbox.id)
        .await
        .expect("create default folders")
        .into_iter()
        .find(|folder| folder.name == "INBOX")
        .expect("find INBOX");
    let (first, second) = tokio::join!(
        store.append(inbox.id, b"1234567", &[], Utc::now()),
        store.append(inbox.id, b"abcdefg", &[], Utc::now())
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(matches!(
        (&first, &second),
        (Err(MailboxStoreError::QuotaExceeded), Ok(_))
            | (Ok(_), Err(MailboxStoreError::QuotaExceeded))
    ));
    let (message_count, used_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::BIGINT, mailbox.used_bytes
         FROM mailbox_messages AS message
         JOIN mailboxes AS mailbox ON mailbox.id = message.mailbox_id
         WHERE message.mailbox_id = $1
         GROUP BY mailbox.used_bytes",
    )
    .bind(mailbox.id)
    .fetch_one(&pool)
    .await
    .expect("read concurrent append accounting");
    assert_eq!(message_count, 1);
    assert_eq!(used_bytes, 7);

    sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(mailbox.id)
        .execute(&pool)
        .await
        .expect("remove test mailbox");
    sqlx::query("DELETE FROM domains WHERE id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .expect("remove test domain");
    sqlx::query("DELETE FROM tenant_memberships WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("remove test membership");
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("remove test user");
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("remove test tenant");
}
