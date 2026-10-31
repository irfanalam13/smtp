ALTER TABLE tenants
    ADD COLUMN max_smtp_message_bytes BIGINT NOT NULL DEFAULT 10485760
        CHECK (max_smtp_message_bytes BETWEEN 1 AND 1073741824);

ALTER TABLE domains
    ADD COLUMN max_smtp_message_bytes BIGINT
        CHECK (max_smtp_message_bytes IS NULL OR
               max_smtp_message_bytes BETWEEN 1 AND 1073741824);
