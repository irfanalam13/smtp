DROP TABLE IF EXISTS tenant_usage_windows;
DROP TABLE IF EXISTS auth_webauthn_credentials;
DROP TABLE IF EXISTS auth_refresh_tokens;
DROP TABLE IF EXISTS auth_mfa_challenges;
DROP TABLE IF EXISTS auth_mfa_recovery_codes;
DROP TABLE IF EXISTS user_identities;
ALTER TABLE users
    DROP COLUMN IF EXISTS totp_confirmed_at,
    DROP COLUMN IF EXISTS totp_secret_encrypted,
    DROP COLUMN IF EXISTS credential_epoch;
