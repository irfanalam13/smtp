use super::*;

impl MailboxStore {
    pub async fn save_rule(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        rule: &MailRule,
    ) -> Result<i64, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        RulePlanner
            .plan(
                std::slice::from_ref(rule),
                RuleContext {
                    direction: rule.direction,
                    from: "",
                    to: "",
                    cc: "",
                    subject: "",
                    body_preview: "",
                    has_attachments: false,
                    size_bytes: 0,
                },
            )
            .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?;
        let direction = match rule.direction {
            RuleDirection::Incoming => "incoming",
            RuleDirection::Outgoing => "outgoing",
            RuleDirection::Both => "both",
        };
        let condition = serde_json::to_value(&rule.condition)
            .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?;
        let actions = serde_json::to_value(&rule.actions)
            .map_err(|_| MailboxStoreError::InvalidWorkflowValue)?;
        let mut transaction = self.pool.begin().await?;
        let version = sqlx::query_scalar::<_, i64>(
            "INSERT INTO mailbox_filters
                (id, mailbox_id, name, direction, enabled, priority, stop_processing,
                 conditions, actions, version)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)
             ON CONFLICT (mailbox_id, id) DO UPDATE
             SET direction = EXCLUDED.direction, enabled = EXCLUDED.enabled,
                 priority = EXCLUDED.priority, stop_processing = EXCLUDED.stop_processing,
                 conditions = EXCLUDED.conditions, actions = EXCLUDED.actions,
                 version = mailbox_filters.version + 1, updated_at = NOW()
             RETURNING version",
        )
        .bind(rule.id)
        .bind(mailbox_id)
        .bind(format!("rule-{}", rule.id))
        .bind(direction)
        .bind(rule.enabled)
        .bind(rule.priority)
        .bind(rule.stop_processing)
        .bind(&condition)
        .bind(&actions)
        .fetch_one(&mut *transaction)
        .await?;
        let snapshot = serde_json::json!({
            "direction": direction,
            "enabled": rule.enabled,
            "priority": rule.priority,
            "stop_processing": rule.stop_processing,
            "conditions": condition,
            "actions": actions,
        });
        sqlx::query(
            "INSERT INTO mailbox_filter_versions (mailbox_id, filter_id, version, snapshot)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(mailbox_id)
        .bind(rule.id)
        .bind(version)
        .bind(snapshot)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(version)
    }

    pub async fn list_rules(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
    ) -> Result<Vec<MailboxRuleRecord>, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Read,
        )
        .await?;
        let rows = sqlx::query(
            "SELECT id, mailbox_id, version, direction, enabled, priority, stop_processing,
                    conditions, actions
             FROM mailbox_filters WHERE mailbox_id = $1
             ORDER BY priority, id",
        )
        .bind(mailbox_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(MailboxRuleRecord {
                    id: row.try_get("id")?,
                    mailbox_id: row.try_get("mailbox_id")?,
                    version: row.try_get("version")?,
                    direction: row.try_get("direction")?,
                    enabled: row.try_get("enabled")?,
                    priority: row.try_get("priority")?,
                    stop_processing: row.try_get("stop_processing")?,
                    conditions: row.try_get("conditions")?,
                    actions: row.try_get("actions")?,
                })
            })
            .collect()
    }

    pub async fn delete_rule(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        mailbox_id: Uuid,
        rule_id: Uuid,
    ) -> Result<bool, MailboxStoreError> {
        authorize_mailbox(
            &self.pool,
            tenant_id,
            user_id,
            mailbox_id,
            MailboxPermission::Manage,
        )
        .await?;
        let result = sqlx::query("DELETE FROM mailbox_filters WHERE mailbox_id = $1 AND id = $2")
            .bind(mailbox_id)
            .bind(rule_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }
}
