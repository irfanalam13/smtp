# Phase 129 — Enterprise Administration and Lifecycle

Status: **Enterprise organizations, policies, regions, and licensing are not implemented**

## Goal and scope

Define secure enterprise administration across organizations, users, regions,
policy, lifecycle, licensing, support, and audit. Administrative power must
be least-privilege, scoped, attributable, and separated from platform
operator access.

## Repository baseline

Phases 71–100 are incomplete, including tenant/workspace management and
production RBAC. Phases 101–110 also remain partial. There is no production
enterprise admin console, region manager, policy engine, license service, or
tenant lifecycle automation.

## Recommended design

Use tenant-scoped roles and approvals, explicit ownership transfer, invite
expiry, lifecycle states, data export/deletion workflows, residency controls,
license entitlements, and break-glass operator processes. Separate customer
administrators from platform operators. Audit changes to region, retention,
identity, security, billing, and licensing. Enforce policy at authoritative
service boundaries rather than relying on UI controls.

## Remaining work and why

Complete tenant boundaries, identity lifecycle, RBAC/ABAC, audit, data
residency, and billing foundations before administration UI/API. This remains
open because building a broad privileged console on incomplete authorization
and audit would create cross-tenant exposure risk.

## Acceptance criteria

- Admin operations are tenant-scoped, permission-checked server-side, and
  audited with actor, reason, outcome, and approval where required.
- Platform support access is separate, just-in-time, time-bounded, and
  customer-visible where appropriate.
- Tenant lifecycle includes provisioning, suspension, export, deletion,
  retention exceptions, and recovery with explicit state transitions.
- Region and policy changes are validated against data residency and restore
  capabilities.
- License enforcement is consistent across API and worker operations and
  handles grace/renewal states safely.

## Research references

- [Phase 73 Organization and Workspace Management](phase-73-organization-workspace-management.md)
- [Phase 75 RBAC and Permission Engine](phase-75-rbac-permission-engine.md)
- [Phase 93 Tenant Administration](phase-93-tenant-administration.md)
- [Phase 94 Super Admin Platform](phase-94-super-admin-platform.md)
