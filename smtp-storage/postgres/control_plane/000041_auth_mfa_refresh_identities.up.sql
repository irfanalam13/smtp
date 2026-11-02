ALTER TABLE users
    ADD COLUMN credential_epoch INTEGER NOT NULL DEFAULT 1
        CHECK (credential_epoch >= 1),
    ADD COLUMN totp_secret_encrypted BYTEA
        CHECK (totp_secret_encrypted IS NULL OR octet_length(totp_secret_encrypted) BETWEEN 28 AND 512),
    ADD COLUMN totp_confirmed_at TIMESTAMPTZ;

CREATE TABLE user_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issuer TEXT NOT NULL CHECK (length(issuer) BETWEEN 1 AND 2048),
    subject TEXT NOT NULL CHECK (length(subject) BETWEEN 1 AND 255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (issuer, subject)
);

CREATE INDEX user_identities_user_idx ON user_identities (user_id);

INSERT INTO user_identities (user_id, issuer, subject)
SELECT id, identity_issuer, external_subject
FROM users
WHERE identity_issuer IS NOT NULL AND identity_issuer <> ''
ON CONFLICT (issuer, subject) DO NOTHING;

CREATE TABLE auth_mfa_recovery_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(code_hash) = 32),
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX auth_mfa_recovery_codes_user_idx
    ON auth_mfa_recovery_codes (user_id)
    WHERE used_at IS NULL;

CREATE TABLE auth_mfa_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    purpose TEXT NOT NULL CHECK (purpose IN ('login', 'webauthn_register', 'webauthn_login')),
    payload BYTEA,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT auth_mfa_challenges_membership_fk
        FOREIGN KEY (tenant_id, user_id)
        REFERENCES tenant_memberships (tenant_id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX auth_mfa_challenges_expiry_idx
    ON auth_mfa_challenges (expires_at)
    WHERE consumed_at IS NULL;

CREATE TABLE auth_refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    session_id UUID REFERENCES auth_sessions(id) ON DELETE SET NULL,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    reused_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT auth_refresh_tokens_membership_fk
        FOREIGN KEY (tenant_id, user_id)
        REFERENCES tenant_memberships (tenant_id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX auth_refresh_tokens_family_idx
    ON auth_refresh_tokens (family_id, created_at DESC);

CREATE TABLE auth_webauthn_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BYTEA NOT NULL UNIQUE CHECK (octet_length(credential_id) BETWEEN 16 AND 1024),
    public_key BYTEA NOT NULL CHECK (octet_length(public_key) BETWEEN 32 AND 8192),
    sign_count BIGINT NOT NULL DEFAULT 0 CHECK (sign_count >= 0),
    aaguid BYTEA,
    name TEXT NOT NULL DEFAULT 'passkey' CHECK (length(name) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE TABLE tenant_usage_windows (
    tenant_id UUID PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
    window_started_at TIMESTAMPTZ NOT NULL,
    window_seconds INTEGER NOT NULL DEFAULT 3600 CHECK (window_seconds BETWEEN 60 AND 86400),
    messages_used INTEGER NOT NULL DEFAULT 0 CHECK (messages_used >= 0),
    message_limit INTEGER NOT NULL DEFAULT 1000 CHECK (message_limit >= 0),
    max_message_bytes INTEGER NOT NULL DEFAULT 10485760 CHECK (max_message_bytes BETWEEN 1024 AND 10485760),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
