DROP INDEX IF EXISTS webhook_deliveries_expired_lease_idx;
ALTER TABLE webhook_deliveries DROP COLUMN IF EXISTS lease_until;
