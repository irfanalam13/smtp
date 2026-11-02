DROP TRIGGER IF EXISTS mailbox_message_retention_audit ON mailbox_messages;
DROP FUNCTION IF EXISTS audit_mailbox_message_deletion();
