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

Future OIDC and workload identity integrations must preserve these contracts:
issuer, audience, expiry, signature, key identity, and revocation state must be
validated before creating an identity.
