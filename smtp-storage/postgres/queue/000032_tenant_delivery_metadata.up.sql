-- Per-message delivery metadata.  Columns remain nullable for pre-existing
-- queue rows; new API submissions always populate them.
ALTER TABLE mail_messages
    ADD COLUMN tenant_id UUID REFERENCES tenants(id) ON DELETE RESTRICT,
    ADD COLUMN sender_domain TEXT,
    ADD COLUMN delivery_mode TEXT NOT NULL DEFAULT 'direct_mx'
        CHECK (delivery_mode IN ('direct_mx', 'relay', 'mailpit')),
    ADD COLUMN dkim_key_ref TEXT;

ALTER TABLE mail_messages
    ADD CONSTRAINT mail_messages_sender_domain_valid
        CHECK (sender_domain IS NULL OR length(sender_domain) BETWEEN 1 AND 253),
    ADD CONSTRAINT mail_messages_dkim_key_ref_valid
        CHECK (dkim_key_ref IS NULL OR length(dkim_key_ref) BETWEEN 1 AND 512);

CREATE INDEX mail_messages_tenant_delivery_idx
    ON mail_messages (tenant_id, delivery_mode, created_at DESC)
    WHERE tenant_id IS NOT NULL;
