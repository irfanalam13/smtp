DROP INDEX IF EXISTS mail_messages_tenant_delivery_idx;
ALTER TABLE mail_messages
    DROP CONSTRAINT IF EXISTS mail_messages_dkim_key_ref_valid,
    DROP CONSTRAINT IF EXISTS mail_messages_sender_domain_valid,
    DROP COLUMN IF EXISTS dkim_key_ref,
    DROP COLUMN IF EXISTS delivery_mode,
    DROP COLUMN IF EXISTS sender_domain,
    DROP COLUMN IF EXISTS tenant_id;
