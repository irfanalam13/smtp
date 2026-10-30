# Phase 75 — RBAC and Permission Engine

Status: **Tenant-scoped fixed roles and mailbox permissions exist; general policy engine pending**

## Repository baseline

Tenant memberships currently use fixed `owner`, `admin`, and `member` roles.
Mailbox access adds explicit mailbox-scoped permissions. The Go API uses
authenticated principals and explicit permission checks; legacy tenant-only
API keys do not gain mailbox access. There is no general role catalog,
custom-role system, workspace inheritance, or centralized policy engine.

## Recommended authorization model

Keep authorization default-deny and separate authentication, tenant
membership, action permission, and resource ownership. Model a small
tenant-scoped role/permission mapping first; add workspace/resource scope only
when Phase 73 defines it. Enforce the policy at the owning API/repository
boundary, and keep mailbox-specific grants explicit. Do not implement
user-supplied policy code or an unbounded policy language.

Every decision is based on a server-derived principal and resource tenant.
Roles and scopes are not accepted from request bodies. Any cached decision
must have a short bound and be invalidated/versioned on membership, role,
resource, and tenant-status changes. Denials and privileged changes should be
auditable without recording secrets or message content.

## Remaining work and why

Define a permission vocabulary and matrix for tenant, workspace, domain,
mailbox, API-key, billing, audit, and platform-admin resources; document
inheritance, service accounts, delegated administration, emergency access,
cache invalidation, and audit policy. Then migrate roles, add authorization
helpers at resource boundaries, and add positive/negative policy tests.

This remains open because the current fixed membership roles and mailbox
permissions cover only selected resources. A generic engine added before
resource ownership and workspace semantics are settled could create
inconsistent privilege checks or broaden tenant access accidentally.

## Acceptance criteria

- Every protected operation has an explicit required action and resource
  scope; no implicit tenant-wide read privilege exists.
- Cross-tenant and cross-workspace access is denied even with guessed IDs.
- Membership removal, role change, and key revocation invalidate effective
  access within a documented bound.
- Role grants and sensitive administrative actions are transactional and
  auditable.
- Policy tests cover allow, deny, missing principal, stale role, service
  account, and resource-not-found/existence-disclosure behavior.

## Research references

- [Tenant isolation strategy](phase-72-multi-tenant-database.md)
- [Organization/workspace model](phase-73-organization-workspace-management.md)
- [Repository security requirements](security-requirements.md)
- [Repository authentication and authorization](../security/authentication-authorization.md)
