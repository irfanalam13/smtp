ALTER TABLE smtp_credentials
    ADD COLUMN user_id UUID;

ALTER TABLE smtp_credentials
    ADD CONSTRAINT smtp_credentials_membership_fk
        FOREIGN KEY (tenant_id, user_id)
        REFERENCES tenant_memberships (tenant_id, user_id)
        ON DELETE CASCADE;

CREATE INDEX smtp_credentials_user_idx
    ON smtp_credentials (tenant_id, user_id)
    WHERE user_id IS NOT NULL;
