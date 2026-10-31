# smtp-auth

`smtp-auth` is the shared identity and security library used by the Go API. It
keeps authentication primitives separate from authorization policy and avoids
duplicating token, secret, and permission handling in individual services.

## Implemented

- Deny-by-default role-to-permission mapping and permission checks.
- Cryptographically random opaque secrets, SHA-256 verifier helpers, constant
  time CSRF comparison, and PKCE verifier generation.
- Bounded Argon2id password hash/verify primitives. They are not yet connected
  to a user/password login or recovery flow.
- Generic OIDC Authorization Code login using state, nonce, PKCE, issuer,
  audience, signature, expiration, and issued-at validation.

The control-plane API consumes this module for its authorization checks,
generic OIDC login, and browser-session secret/CSRF handling. Browser sessions
are persisted by the API in PostgreSQL because HTTP routing and tenancy
validation belong at that service boundary.

## Current identity behavior

Browser sign-in supports one server-configured OIDC provider and requires the
identity to already match a user by exact issuer and subject, with an active
tenant membership. It does not link accounts by email. The API also retains
its separate scoped API-key and OIDC resource-server paths; SMTP credentials
remain separate submission identities.

This module does not yet provide account registration, password login,
verification or recovery, refresh tokens, provider discovery/management,
GitHub OAuth identity lookup, MFA/TOTP, WebAuthn, recovery codes, account
linking, invitations, or service-account lifecycle. Do not present these as
implemented.
