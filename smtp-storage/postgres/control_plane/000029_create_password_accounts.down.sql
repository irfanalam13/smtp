DROP TABLE auth_email_tokens;
DROP INDEX users_email_unique_idx;
ALTER TABLE users
    DROP CONSTRAINT users_password_hash_length,
    DROP CONSTRAINT users_email_lowercase,
    DROP COLUMN email_verified_at,
    DROP COLUMN password_hash,
    DROP COLUMN email;
