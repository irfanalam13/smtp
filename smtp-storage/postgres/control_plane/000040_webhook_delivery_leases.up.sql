ALTER TABLE webhook_deliveries
    ADD COLUMN lease_until TIMESTAMPTZ;

CREATE INDEX webhook_deliveries_expired_lease_idx
    ON webhook_deliveries (lease_until)
    WHERE state = 'delivering';
