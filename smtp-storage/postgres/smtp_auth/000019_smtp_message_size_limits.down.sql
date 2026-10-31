ALTER TABLE domains
    DROP COLUMN max_smtp_message_bytes;

ALTER TABLE tenants
    DROP COLUMN max_smtp_message_bytes;
