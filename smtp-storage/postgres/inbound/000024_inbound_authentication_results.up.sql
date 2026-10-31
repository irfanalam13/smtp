ALTER TABLE inbound_messages
    DROP CONSTRAINT IF EXISTS inbound_messages_status_check;

ALTER TABLE inbound_messages
    ADD CONSTRAINT inbound_messages_status_check
        CHECK (status IN ('stored', 'quarantined', 'deleted')),
    ADD COLUMN authentication_results JSONB
        CHECK (authentication_results IS NULL OR jsonb_typeof(authentication_results) = 'object');

CREATE INDEX inbound_messages_quarantine_idx
    ON inbound_messages (tenant_id, received_at DESC, id)
    WHERE status = 'quarantined';
