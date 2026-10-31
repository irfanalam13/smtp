CREATE TABLE smtp_credentials (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    username TEXT NOT NULL CHECK (length(username) BETWEEN 1 AND 128),
    password_hash TEXT NOT NULL CHECK (length(password_hash) BETWEEN 1 AND 512),
    password_hash_version SMALLINT NOT NULL DEFAULT 1
        CHECK (password_hash_version = 1),
    password_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, username)
);
