ALTER TABLE inbound_messages
    DROP COLUMN received_over_tls,
    DROP COLUMN received_by,
    DROP COLUMN received_from_ip;
