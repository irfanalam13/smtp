# API key and SMTP credential lifecycle

API keys are generated with `crypto/rand` as `sp_` followed by 32 random bytes encoded with base64url. Only a SHA-256 verifier is stored; the full secret is returned once at creation and cannot be recovered.

`POST /v1/api-keys` accepts either an absolute `expires_at` or one of these safer relative values:

- `1_week`
- `1_month`
- `3_months`
- `6_months`
- `1_year`

Expired and revoked keys are rejected during authentication. A daily lifecycle worker sends a warning after 83 days without use and revokes the key after 90 days. It uses the creator's verified account email where available. The worker is safe to run on multiple API instances because the warning update is conditional and revocation is idempotent.

SMTP credentials remain separate from API keys and are managed with `POST /v1/smtp-credentials`, rotation, revocation, and expiry. The SMTP username/password is also returned only on creation or rotation; the password is stored as a password hash.
