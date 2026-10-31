DROP INDEX inbound_messages_quarantine_idx;

UPDATE inbound_messages SET status = 'stored' WHERE status = 'quarantined';
ALTER TABLE inbound_messages DROP COLUMN authentication_results;
ALTER TABLE inbound_messages
    DROP CONSTRAINT inbound_messages_status_check,
    ADD CONSTRAINT inbound_messages_status_check
        CHECK (status IN ('stored', 'deleted'));
