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

- Browser OIDC authorization-code/PKCE flow, session lifecycle, and enforced
  MFA policy.
- Production identity provisioning/deprovisioning workflows.
- Production API-key persistence and AWS Secrets Manager integration.
- mTLS or workload identity for service-to-service calls.
- AWS KMS/Secrets Manager-backed key management and certificate provisioning.
- Durable audit persistence in the control-plane runtime.

These are tracked as deployment requirements rather than represented as
completed controls.

## Remaining implementation

The API's OIDC verifier is a resource-server slice, not a complete production
identity platform. Durable credentials and audit storage, EKS workload
identity, AWS KMS/Secrets Manager, certificate lifecycle, and security
operations remain deferred. See the [phase implementation
status](../phase-status.md) under Phase 07 for the remaining controls and
release conditions.
