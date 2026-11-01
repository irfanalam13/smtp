CREATE TABLE auth_email_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deduplication_key TEXT NOT NULL UNIQUE CHECK (length(deduplication_key) BETWEEN 16 AND 200),
    recipient TEXT NOT NULL CHECK (length(recipient) BETWEEN 3 AND 320),
    subject TEXT NOT NULL CHECK (length(subject) BETWEEN 1 AND 998),
    body_ciphertext BYTEA NOT NULL CHECK (octet_length(body_ciphertext) > 28),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'sending', 'sent', 'dead_letter')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_until TIMESTAMPTZ,
    last_error TEXT CHECK (last_error IS NULL OR length(last_error) <= 500),
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX auth_email_outbox_ready_idx
    ON auth_email_outbox (available_at, created_at, id)
    WHERE status IN ('pending', 'sending');

CREATE INDEX auth_email_outbox_status_idx
    ON auth_email_outbox (status, updated_at);
