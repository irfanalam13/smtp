CREATE TABLE IF NOT EXISTS mailboxes (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE RESTRICT,
    local_part TEXT NOT NULL CHECK (length(local_part) BETWEEN 1 AND 64),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (domain_id, local_part)
);

CREATE TABLE IF NOT EXISTS inbound_messages (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    mailbox_id UUID NOT NULL REFERENCES mailboxes(id) ON DELETE RESTRICT,
    envelope_sender TEXT NOT NULL CHECK (length(envelope_sender) <= 320),
    raw_message BYTEA NOT NULL,
    raw_size BIGINT NOT NULL CHECK (raw_size BETWEEN 0 AND 10485760),
    subject TEXT CHECK (subject IS NULL OR length(subject) <= 998),
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL DEFAULT 'stored' CHECK (status IN ('stored', 'deleted')),
    idempotency_key BYTEA NOT NULL CHECK (octet_length(idempotency_key) = 32),
    UNIQUE (mailbox_id, idempotency_key)
);

CREATE TABLE IF NOT EXISTS inbound_recipients (
    message_id UUID NOT NULL REFERENCES inbound_messages(id) ON DELETE RESTRICT,
    recipient TEXT NOT NULL CHECK (length(recipient) BETWEEN 1 AND 320),
    PRIMARY KEY (message_id, recipient)
);

CREATE INDEX IF NOT EXISTS mailboxes_tenant_idx ON mailboxes (tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS inbound_messages_mailbox_idx ON inbound_messages (mailbox_id, received_at DESC, id);
CREATE INDEX IF NOT EXISTS inbound_messages_tenant_idx ON inbound_messages (tenant_id, received_at DESC);
