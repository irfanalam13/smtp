use super::*;

impl MailboxStore {
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
}
