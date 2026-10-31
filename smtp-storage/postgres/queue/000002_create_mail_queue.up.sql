CREATE TABLE IF NOT EXISTS mail_messages (
    id UUID PRIMARY KEY,
    envelope_sender TEXT NOT NULL CHECK (length(envelope_sender) BETWEEN 1 AND 320),
    body BYTEA NOT NULL,
    body_size BIGINT NOT NULL CHECK (body_size BETWEEN 0 AND 10485760),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS mail_recipients (
    id BIGSERIAL PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES mail_messages(id) ON DELETE CASCADE,
    recipient TEXT NOT NULL CHECK (length(recipient) BETWEEN 1 AND 320),
    UNIQUE (message_id, recipient)
);

CREATE TABLE IF NOT EXISTS mail_queue (
    message_id UUID PRIMARY KEY REFERENCES mail_messages(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('queued', 'processing', 'retry_scheduled', 'simulated_succeeded', 'terminal_failed')),
    next_attempt_at TIMESTAMPTZ NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    lease_owner TEXT CHECK (lease_owner IS NULL OR length(lease_owner) BETWEEN 1 AND 128),
    lease_token UUID,
    lease_expires_at TIMESTAMPTZ,
    last_error_category TEXT CHECK (last_error_category IS NULL OR last_error_category IN ('temporary', 'permanent', 'lease_lost', 'cancelled', 'internal')),
    last_error_detail TEXT CHECK (last_error_detail IS NULL OR length(last_error_detail) <= 512),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK ((state = 'processing') = (lease_token IS NOT NULL AND lease_expires_at IS NOT NULL AND lease_owner IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS mail_idempotency (
    scope TEXT NOT NULL CHECK (length(scope) BETWEEN 1 AND 128),
    idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 256),
    fingerprint BYTEA NOT NULL CHECK (octet_length(fingerprint) = 32),
    message_id UUID NOT NULL REFERENCES mail_messages(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (scope, idempotency_key)
);

CREATE TABLE IF NOT EXISTS mail_attempts (
    id BIGSERIAL PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES mail_messages(id) ON DELETE CASCADE,
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    outcome TEXT NOT NULL CHECK (outcome IN ('simulated_success', 'failure')),
    error_category TEXT CHECK (error_category IS NULL OR error_category IN ('temporary', 'permanent', 'lease_lost', 'cancelled', 'internal')),
    diagnostic TEXT CHECK (diagnostic IS NULL OR length(diagnostic) <= 512)
);

CREATE INDEX IF NOT EXISTS mail_queue_due_idx ON mail_queue (next_attempt_at, created_at) WHERE state IN ('queued', 'retry_scheduled');
CREATE INDEX IF NOT EXISTS mail_queue_lease_idx ON mail_queue (lease_expires_at) WHERE state = 'processing';
CREATE INDEX IF NOT EXISTS mail_attempts_message_idx ON mail_attempts (message_id, attempt_number);
