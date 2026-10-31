DROP INDEX IF EXISTS api_keys_lifecycle_idx;
ALTER TABLE api_keys DROP COLUMN IF EXISTS inactivity_warning_sent_at;
