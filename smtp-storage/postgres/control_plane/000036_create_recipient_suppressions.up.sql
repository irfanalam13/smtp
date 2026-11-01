CREATE TABLE recipient_suppressions (
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    recipient TEXT NOT NULL CHECK (length(recipient) BETWEEN 3 AND 320),
    reason TEXT NOT NULL CHECK (reason IN ('hard_bounce', 'complaint', 'manual')),
    source_event_id UUID,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, recipient)
);

CREATE INDEX recipient_suppressions_active_idx
    ON recipient_suppressions (tenant_id, recipient)
    WHERE active = TRUE;

CREATE UNIQUE INDEX recipient_suppressions_active_normalized_idx
    ON recipient_suppressions (tenant_id, lower(recipient))
    WHERE active = TRUE;
