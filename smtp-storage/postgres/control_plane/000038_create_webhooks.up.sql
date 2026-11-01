CREATE TABLE webhook_subscriptions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL CHECK (length(endpoint) BETWEEN 1 AND 2048),
    secret BYTEA NOT NULL CHECK (octet_length(secret) >= 32),
    event_types TEXT[] NOT NULL CHECK (cardinality(event_types) > 0),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX webhook_subscriptions_tenant_idx
    ON webhook_subscriptions (tenant_id, created_at DESC);

CREATE TABLE webhook_deliveries (
    id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL REFERENCES webhook_subscriptions(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    event_id UUID NOT NULL,
    event_type TEXT NOT NULL CHECK (length(event_type) BETWEEN 1 AND 128),
    payload JSONB NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    state TEXT NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending', 'delivering', 'delivered', 'retry', 'dead_letter')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error TEXT,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (subscription_id, event_id)
);

CREATE INDEX webhook_deliveries_ready_idx
    ON webhook_deliveries (next_attempt_at, created_at)
    WHERE state IN ('pending', 'retry');
