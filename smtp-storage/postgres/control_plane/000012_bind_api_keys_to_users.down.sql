DROP INDEX IF EXISTS api_keys_creator_idx;

ALTER TABLE api_keys
    DROP CONSTRAINT IF EXISTS api_keys_creator_membership_fk,
    DROP COLUMN IF EXISTS created_by_user_id;
