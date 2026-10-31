CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    csrf_hash BYTEA NOT NULL CHECK (octet_length(csrf_hash) = 32),
    user_agent TEXT NOT NULL DEFAULT '' CHECK (length(user_agent) <= 512),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    idle_expires_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    CONSTRAINT auth_sessions_expiry_order CHECK (idle_expires_at <= expires_at),
    CONSTRAINT auth_sessions_membership_fk
        FOREIGN KEY (tenant_id, user_id)
        REFERENCES tenant_memberships (tenant_id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX auth_sessions_user_active_idx
    ON auth_sessions (user_id, last_used_at DESC)
    WHERE revoked_at IS NULL;

CREATE INDEX auth_sessions_expiry_idx
    ON auth_sessions (expires_at)
    WHERE revoked_at IS NULL;
