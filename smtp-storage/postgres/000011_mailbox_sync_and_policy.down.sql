DROP TRIGGER IF EXISTS mailbox_draft_revision_change_journal ON mailbox_draft_revisions;
DROP TRIGGER IF EXISTS mailbox_custom_properties_change_journal ON mailbox_messages;
DROP TRIGGER IF EXISTS mailbox_label_assignment_change_journal
    ON mailbox_message_label_assignments;
DROP FUNCTION IF EXISTS record_mailbox_metadata_change();
