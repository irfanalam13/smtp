# Phase 07 security architecture

## Implemented boundary

The current control plane authenticates either a scoped API key or, only when
`CONTROL_PLANE_DEV_MODE=true`, a development subject header. The development
adapter is rejected when `APP_ENVIRONMENT` is `production` or `prod`. No
production OIDC issuer is configured yet, so the adapter is not a production
identity system.

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

- OIDC issuer and audience validation.
- Production API-key persistence and external secret management.
- mTLS or workload identity for service-to-service calls.
- KMS/HSM-backed encryption keys and certificate provisioning.
- Durable audit persistence in the control-plane runtime.

These are tracked as deployment requirements rather than represented as
completed controls.

## Remaining implementation

The local security foundation is intentionally not a production identity
platform. OIDC, durable credentials and audit storage, workload identity,
KMS/HSM, certificate lifecycle, and security operations remain deferred because
their providers and deployment ownership are not selected. See the [phase
implementation status](../phase-status.md) under Phase 07 for the reason and
the Phase 08/10 conditions for applying each control.
