CREATE TABLE feedback_events (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    recipient TEXT NOT NULL CHECK (length(recipient) BETWEEN 3 AND 320),
    event_type TEXT NOT NULL CHECK (event_type IN ('bounce', 'complaint')),
    status TEXT,
    diagnostic TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, event_type, recipient, status, diagnostic)
);

CREATE INDEX feedback_events_tenant_idx
    ON feedback_events (tenant_id, created_at DESC);
