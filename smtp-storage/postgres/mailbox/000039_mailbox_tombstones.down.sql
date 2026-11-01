DROP TRIGGER IF EXISTS mailbox_message_tombstone ON mailbox_messages;
DROP FUNCTION IF EXISTS record_mailbox_message_tombstone();
DROP INDEX IF EXISTS mailbox_message_tombstones_tenant_idx;
DROP TABLE IF EXISTS mailbox_message_tombstones;
