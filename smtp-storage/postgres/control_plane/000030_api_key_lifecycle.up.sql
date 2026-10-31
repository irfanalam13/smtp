ALTER TABLE api_keys
    ADD COLUMN IF NOT EXISTS inactivity_warning_sent_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS api_keys_lifecycle_idx
    ON api_keys (revoked_at, last_used_at, inactivity_warning_sent_at)
    WHERE revoked_at IS NULL;
