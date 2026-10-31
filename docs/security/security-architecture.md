# Phase 07 security architecture

## Implemented boundary

The current control plane authenticates a scoped API key, an optional
Keycloak OIDC access token for a pre-provisioned user, or—only when
`CONTROL_PLANE_DEV_MODE=true` in an explicitly development environment—a
development subject header. The development adapter is rejected in all other
environments.

Authentication produces a principal with a stable subject, tenant, role, and
scopes. Authorization is evaluated at the HTTP boundary and again by store
operations. The security package applies default deny and requires the resource
tenant to match the authenticated tenant.

API-key secrets are generated with the operating-system CSPRNG, stored only as
SHA-256 verifiers, compared in constant time, scoped, optionally expired, and
revocable. API-key principals are represented as service identities and never
receive human-user privileges outside their explicit scopes.

The `smtp-auth` module is the shared authorization and OIDC/secret primitive
boundary. Browser login uses a configured OIDC Authorization Code flow with
PKCE, state, nonce, issuer/audience/signature/time validation, and exact
issuer-subject identity matching. PostgreSQL-backed browser sessions store
only token verifiers, use HttpOnly/SameSite cookies, rotate on successful
login, enforce idle and absolute expiration, and support per-session and
global revocation. Unsafe cookie-authenticated requests require a CSRF token
and exact same-origin validation. Existing API-key and SMTP credential
authentication remain separate, explicitly scoped paths.

## Trust boundaries

1. Internet clients to the HTTP control plane.
2. HTTP handlers to control-plane state.
3. Control-plane state to PostgreSQL in deployments that replace the current
   in-memory adapter.
4. Operators and deployment configuration to the process.
5. Outbound SMTP delivery to remote MX hosts.

Network location is not an authorization signal. Tenant and resource ownership
come from authenticated state, not request payloads.

## Deferred production integrations

- OIDC user provisioning/deprovisioning, MFA/TOTP, WebAuthn/passkeys, and
  recovery codes. Email/password signup, login, verification, and password
  recovery are implemented for self-service workspace accounts.
- Per-tenant OIDC configuration, GitHub's OAuth identity API, and verified
  production interoperability with named identity providers.
- JWT refresh-token lifecycle and reuse detection; browser sessions currently
  use opaque database-backed credentials rather than refresh tokens.
- Production API-key secret management and AWS Secrets Manager integration.
- mTLS or workload identity for service-to-service calls.
- AWS KMS/Secrets Manager-backed key management and certificate provisioning.
- Distributed authentication rate limiting and durable audit export in
  production operations.

These are tracked as deployment requirements rather than represented as
completed controls.

## Remaining implementation

The API's OIDC verifier and browser OIDC/session path are not a complete
production identity platform. Identity lifecycle, durable audit export, EKS
workload identity, AWS KMS/Secrets Manager, certificate lifecycle, and
security operations remain deferred. See the [phase implementation
status](../phase-status.md) under Phase 07 for the remaining controls and
release conditions.
