ALTER TABLE inbound_messages
    ADD COLUMN received_from_ip INET,
    ADD COLUMN received_by TEXT CHECK (received_by IS NULL OR length(received_by) BETWEEN 1 AND 255),
    ADD COLUMN received_over_tls BOOLEAN;
