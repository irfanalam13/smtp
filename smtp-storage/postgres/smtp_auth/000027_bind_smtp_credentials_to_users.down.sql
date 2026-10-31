DROP INDEX smtp_credentials_user_idx;

ALTER TABLE smtp_credentials
    DROP CONSTRAINT smtp_credentials_membership_fk,
    DROP COLUMN user_id;
