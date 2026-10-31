CREATE TABLE smtp_inbound_rate_limits (
    key_hash BYTEA PRIMARY KEY CHECK (octet_length(key_hash) = 32),
    dimension TEXT NOT NULL CHECK (dimension IN ('ip', 'domain')),
    window_started_at TIMESTAMPTZ NOT NULL,
    accepted_messages INTEGER NOT NULL CHECK (accepted_messages > 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX smtp_inbound_rate_limits_updated_idx
    ON smtp_inbound_rate_limits (updated_at);
