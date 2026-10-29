# ADR 0006: Control-plane authentication and tenant isolation

## Status

Accepted for Milestone 4.

## Decision

The control plane uses a small provider-neutral principal boundary. Production
identity-provider integration is deferred until an OIDC issuer, audience, and
key-discovery configuration is available; no password or custom token-signing
system is introduced. Local development may set
`CONTROL_PLANE_DEV_MODE=true` and identify a test principal with the
`X-Dev-Subject` header. This mode is development-only and must not be enabled
at a deployment boundary.

The application derives the tenant from the principal and never accepts a
tenant identifier from request JSON or query parameters. The service store
checks tenant ownership and scopes before every mutation. PostgreSQL control
plane tables use tenant foreign keys, restrictive deletion, unique constraints,
and tenant indexes. PostgreSQL row-level security is not claimed because the
current Go API does not yet own a configured PostgreSQL pool/session context;
application authorization remains mandatory.
