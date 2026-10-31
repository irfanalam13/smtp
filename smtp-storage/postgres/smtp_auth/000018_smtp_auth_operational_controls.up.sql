CREATE TABLE smtp_credential_audit_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    credential_id UUID NOT NULL REFERENCES smtp_credentials(id) ON DELETE RESTRICT,
    actor_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (action IN ('created', 'rotated', 'revoked')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX smtp_credential_audit_tenant_created_idx
    ON smtp_credential_audit_events (tenant_id, created_at DESC);

CREATE TABLE smtp_auth_rate_limits (
    key_hash BYTEA PRIMARY KEY CHECK (octet_length(key_hash) = 32),
    dimension TEXT NOT NULL CHECK (dimension IN ('ip', 'tenant', 'account')),
    failures INTEGER NOT NULL CHECK (failures > 0),
    window_started_at TIMESTAMPTZ NOT NULL,
    locked_until TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX smtp_auth_rate_limits_updated_idx
    ON smtp_auth_rate_limits (updated_at);
