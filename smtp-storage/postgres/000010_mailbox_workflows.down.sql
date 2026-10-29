DROP TABLE IF EXISTS mailbox_sent_recipient_status;
DROP TABLE IF EXISTS mailbox_sent_recipient_events;
DROP TABLE IF EXISTS mailbox_sent_submissions;
DROP TRIGGER IF EXISTS mailbox_message_change_journal ON mailbox_messages;
DROP FUNCTION IF EXISTS record_mailbox_message_change();
DROP TABLE IF EXISTS mailbox_event_outbox;
DROP TABLE IF EXISTS mailbox_change_log;

DROP TRIGGER IF EXISTS mailbox_draft_revision_immutable ON mailbox_draft_revisions;
DROP TRIGGER IF EXISTS mailbox_draft_revision_release ON mailbox_draft_revisions;
DROP TRIGGER IF EXISTS mailbox_draft_revision_charge ON mailbox_draft_revisions;
DROP FUNCTION IF EXISTS prevent_mailbox_draft_revision_update();
DROP FUNCTION IF EXISTS release_mailbox_draft_revision();
DROP FUNCTION IF EXISTS charge_mailbox_draft_revision();
DROP TABLE IF EXISTS mailbox_draft_revisions;
DROP TABLE IF EXISTS mailbox_drafts;

DROP TABLE IF EXISTS mailbox_archive_entries;
DROP TABLE IF EXISTS mailbox_trash_entries;
DROP TABLE IF EXISTS mailbox_quarantine_entries;
DROP TABLE IF EXISTS mailbox_rule_executions;
DROP TABLE IF EXISTS mailbox_filter_versions;
DROP TABLE IF EXISTS mailbox_filters;

ALTER TABLE mailbox_messages
    DROP CONSTRAINT IF EXISTS mailbox_messages_conversation_mailbox_fk;

DROP INDEX IF EXISTS mailbox_messages_custom_properties_idx;
DROP TRIGGER IF EXISTS mailbox_label_smart_reclassification_guard ON mailbox_labels;
DROP FUNCTION IF EXISTS prevent_assigned_label_becoming_smart();
DROP TRIGGER IF EXISTS mailbox_message_label_assignment_static_only
    ON mailbox_message_label_assignments;
DROP FUNCTION IF EXISTS prevent_smart_label_assignment();
DROP TABLE IF EXISTS mailbox_message_label_assignments;
DROP TABLE IF EXISTS mailbox_labels;
DROP TABLE IF EXISTS mailbox_conversations;

ALTER TABLE mailbox_messages
    DROP CONSTRAINT IF EXISTS mailbox_messages_conversation_mailbox_fk,
    DROP CONSTRAINT IF EXISTS mailbox_messages_folder_mailbox_fk,
    DROP CONSTRAINT IF EXISTS mailbox_messages_mailbox_id_id_key,
    DROP COLUMN IF EXISTS custom_properties,
    DROP COLUMN IF EXISTS mailbox_id;
