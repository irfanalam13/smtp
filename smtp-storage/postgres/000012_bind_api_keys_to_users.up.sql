ALTER TABLE api_keys
    ADD COLUMN created_by_user_id UUID;

ALTER TABLE api_keys
    ADD CONSTRAINT api_keys_creator_membership_fk
        FOREIGN KEY (tenant_id, created_by_user_id)
        REFERENCES tenant_memberships (tenant_id, user_id)
        ON DELETE SET NULL (created_by_user_id);

CREATE INDEX api_keys_creator_idx
    ON api_keys (tenant_id, created_by_user_id)
    WHERE created_by_user_id IS NOT NULL;
