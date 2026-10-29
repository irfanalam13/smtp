CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 160),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    external_subject TEXT NOT NULL UNIQUE CHECK (length(external_subject) BETWEEN 1 AND 255),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tenant_memberships (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, user_id)
);

CREATE TABLE IF NOT EXISTS domains (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    domain TEXT NOT NULL CHECK (length(domain) BETWEEN 1 AND 253),
    status TEXT NOT NULL DEFAULT 'pending_verification'
        CHECK (status IN ('pending_verification', 'verified', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, domain)
);

CREATE TABLE IF NOT EXISTS domain_verification_challenges (
    domain_id UUID PRIMARY KEY REFERENCES domains(id) ON DELETE RESTRICT,
    token_hash BYTEA NOT NULL CHECK (octet_length(token_hash) = 32),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    prefix TEXT NOT NULL UNIQUE CHECK (length(prefix) BETWEEN 8 AND 32),
    secret_hash BYTEA NOT NULL CHECK (octet_length(secret_hash) = 32),
    scopes TEXT[] NOT NULL CHECK (cardinality(scopes) > 0),
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS api_keys_tenant_idx ON api_keys (tenant_id, created_at DESC);
CREATE TABLE IF NOT EXISTS audit_events (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL CHECK (length(action) BETWEEN 1 AND 120),
    target_type TEXT NOT NULL CHECK (length(target_type) BETWEEN 1 AND 80),
    target_id TEXT CHECK (target_id IS NULL OR length(target_id) <= 120),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS audit_events_tenant_idx ON audit_events (tenant_id, created_at DESC);
