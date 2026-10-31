ALTER TABLE users
    ADD COLUMN email TEXT,
    ADD COLUMN password_hash TEXT,
    ADD COLUMN email_verified_at TIMESTAMPTZ,
    ADD CONSTRAINT users_email_lowercase
        CHECK (email IS NULL OR (email = lower(email) AND length(email) BETWEEN 3 AND 320)),
    ADD CONSTRAINT users_password_hash_length
        CHECK (password_hash IS NULL OR length(password_hash) BETWEEN 1 AND 512);

CREATE UNIQUE INDEX users_email_unique_idx
    ON users (email)
    WHERE email IS NOT NULL;

CREATE TABLE auth_email_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    purpose TEXT NOT NULL CHECK (purpose IN ('verify_email', 'reset_password')),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX auth_email_tokens_user_purpose_idx
    ON auth_email_tokens (user_id, purpose, created_at DESC);
