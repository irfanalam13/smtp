ALTER TABLE api_keys
    ADD COLUMN principal_type TEXT NOT NULL DEFAULT 'tenant'
        CHECK (principal_type IN ('tenant', 'user'));

UPDATE api_keys
SET principal_type = 'user'
WHERE created_by_user_id IS NOT NULL;
