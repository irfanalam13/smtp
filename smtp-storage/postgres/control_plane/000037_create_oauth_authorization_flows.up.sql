CREATE TABLE oauth_authorization_flows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL CHECK (provider IN ('oidc', 'google', 'github')),
    state_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(state_hash) = 32),
    nonce_hash BYTEA NOT NULL CHECK (octet_length(nonce_hash) = 32),
    pkce_verifier_hash BYTEA NOT NULL CHECK (octet_length(pkce_verifier_hash) = 32),
    tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT oauth_authorization_flows_expiry CHECK (expires_at > created_at)
);

CREATE INDEX oauth_authorization_flows_expiry_idx
    ON oauth_authorization_flows (expires_at)
    WHERE consumed_at IS NULL;
