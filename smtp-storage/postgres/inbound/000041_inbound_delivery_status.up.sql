CREATE TABLE inbound_delivery_status (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    mailbox_id UUID NOT NULL REFERENCES mailboxes(id) ON DELETE RESTRICT,
    inbound_message_id UUID NOT NULL REFERENCES inbound_messages(id) ON DELETE RESTRICT,
    recipient TEXT NOT NULL CHECK (length(recipient) BETWEEN 1 AND 320),
    action TEXT NOT NULL CHECK (action IN ('delivered', 'delayed', 'failed')),
    status TEXT NOT NULL CHECK (length(status) BETWEEN 5 AND 16),
    reporting_mta TEXT NOT NULL CHECK (length(reporting_mta) BETWEEN 1 AND 255),
    diagnostic_code TEXT CHECK (diagnostic_code IS NULL OR length(diagnostic_code) <= 1000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (inbound_message_id, recipient, action, status)
);

CREATE INDEX inbound_delivery_status_tenant_idx
    ON inbound_delivery_status (tenant_id, created_at DESC);
