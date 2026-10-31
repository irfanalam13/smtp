# Authentication and authorization

The control plane separates authentication from authorization.

`security.Identity` identifies a human or service caller. `security.Authorize`
requires a non-empty subject and tenant, an explicit identity type, a matching
resource tenant, and an explicit permission. Missing, unknown, or malformed
security state denies access.

The current API-key flow:

1. Generate 256 bits with `crypto/rand`.
2. Return the secret only from the create operation.
3. Store a SHA-256 verifier and non-sensitive prefix.
4. Compare verifiers using constant-time comparison.
5. Reject revoked, expired, malformed, or missing credentials.
6. Construct a service identity with only the key's scopes.

The development subject header is intentionally limited to local development.
It must not be enabled with `APP_ENVIRONMENT=production` or `prod`.

OIDC access-token verification validates issuer, audience, expiry, issued-at,
signature, and the provider's bearer-token type before resolving a principal.
Browser OIDC login separately validates the ID token for its authentication
purpose and never accepts that ID token as an API access token. Both flows
resolve users by exact issuer and subject and require an active membership;
email equality does not link identities.

The shared `smtp-auth` module owns permission mapping and reusable
authentication primitives. The API owns tenant-aware identity resolution,
PostgreSQL browser-session persistence, cookie delivery, CSRF/origin checks,
and security audit events. Browser-session secrets are opaque and only their
SHA-256 verifiers are persisted. Cookies are HttpOnly for the session secret,
Secure outside local HTTP development, SameSite=Lax, and rotated by issuing a
new session after successful login. Idle and absolute expiration, targeted
revocation, logout, and logout-all are implemented.

Password accounts use Argon2id verifiers through `smtp-auth`. Signup creates a
workspace and owner membership, then requires email verification before
password login. Verification and password-reset links use hashed,
single-use, expiring tokens; resetting a password revokes existing browser
sessions. Account email delivery is server-side SMTP and is disabled until
configured. Password login does not issue browser-readable JWT credentials.

## SMTP submission credentials

The SMTP AUTH PLAIN verifier is separate from control-plane API-key
authentication. It accepts a tenant-qualified authcid in the form
`<tenant UUID>:<username>` and only authenticates credentials for an active
tenant. Usernames are normalized to lowercase ASCII. Passwords must be 12–512
bytes and are stored only as Argon2id PHC hashes with a recorded hash-parameter
version. Provisioning rotates an existing tenant/username credential;
revocation and expiry are checked on each authentication.

AUTH is available only after TLS. The verifier performs dummy Argon2 work for
missing/malformed identities, and does not log usernames or passwords.
Per-IP, per-tenant, and per-account throttling is currently bounded but
process-local; it is not a distributed production abuse-control system.
Credential provisioning/revocation is exposed to Rust callers only and does
not yet have an administrative HTTP API.

## Remaining identity capabilities

OIDC user provisioning, JWT refresh-token rotation and reuse detection,
MFA/TOTP, WebAuthn/passkeys, recovery codes, account linking, invitations,
per-tenant provider management, and GitHub's OAuth identity API are not
implemented. Generic OIDC login is not a guarantee of verified interoperability
with every named provider. OIDC provider secrets must remain server-side.
