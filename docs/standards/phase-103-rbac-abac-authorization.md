# Phase 103 — RBAC and ABAC Authorization

Status: **Limited tenant roles/scopes and default-deny checks; policy engine and comprehensive enforcement pending**

## Repository baseline

The control plane supports a small set of membership roles, scoped API keys,
and tenant/resource ownership checks. There is no general policy language,
custom role lifecycle, attribute-based authorization (ABAC), delegated
administration, or verified enforcement at every service/storage boundary.

Keycloak-authenticated API requests resolve the user's active tenant
membership from PostgreSQL and derive only the existing owner/admin/member
scopes from the database role. Token-provided roles or tenant claims do not
grant application permissions. This is a fixed mapping, not a general RBAC or
ABAC engine.

## Recommended design

Define a small, versioned permission vocabulary and resource/action matrix
before adding a policy engine. Use RBAC for stable job functions and ABAC
attributes only for documented contextual restrictions (tenant, resource
ownership, authentication strength, environment, or approved network
identity). Default deny; authorize every object/property/action server-side;
never accept role or tenant claims from mutable request bodies. Make policy
decisions explainable and auditable without logging secrets or message
content. Apply database constraints and RLS only as defense in depth after
connection-pooling/session-context semantics are proven.

## Remaining work and why

Inventory API operations and resources; approve role/permission semantics;
implement policy evaluation, delegation, change review, cache invalidation,
and audit; then add cross-tenant and privilege-escalation tests across API,
worker, and database layers. This remains open because the platform lacks
production identity, complete durable resources, and an approved permission
matrix; generic policy flexibility now would expand the attack surface.

## Acceptance criteria

- A reviewed permission matrix covers read/write/admin/export/credential and
  privileged operations for each resource.
- All API, background-job, and database access paths enforce the same tenant
  and resource boundary.
- Default-deny behavior and object/property-level authorization have negative
  tests.
- Role/policy updates are versioned, audited, and take effect within a
  documented cache/revocation bound.
- Delegation and operator access cannot silently elevate tenant privileges.

## Research references

- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [Phase 75 RBAC and Permission Engine](phase-75-rbac-permission-engine.md)
- [Phase 07 security architecture](../security/security-architecture.md)
