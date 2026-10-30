# Phase 73 — Organization and Workspace Management

Status: **Tenant membership foundation exists; organization/workspace workflows pending**

## Repository baseline

The existing `tenants` table is a natural customer-organization and security
boundary. `tenant_memberships` currently binds a user to a tenant with the
fixed roles `owner`, `admin`, and `member`. Mailboxes and domains belong to
tenants. There are no first-class workspace, invitation, ownership-transfer,
or organization-settings workflows.

## Recommended model

Reuse tenant as the organization/billing boundary. Add a workspace as a
tenant-owned resource only if product requirements need resource segmentation
or delegated administration. A workspace has a composite `(id, tenant_id)`
identity; children retain `tenant_id` and may include `workspace_id`.
Workspace membership cannot expand tenant membership or access another
workspace by default. Keep an organization-wide owner and explicit transfer
workflow; do not make a workspace admin a tenant owner implicitly.

Invitations should be opaque, high-entropy, single-use credentials stored only
as hashes, scoped to one tenant and intended role, with expiry, revocation,
rate limits, and audit events. Acceptance must revalidate tenant status,
inviter authority, and invitation state transactionally. Email is a delivery
channel, not proof of identity. Membership removal must revoke effective
resource grants and active sessions/tokens according to Phase 74 policy.

## Remaining work and why

Approve whether workspaces are needed and define their resource scope, role
inheritance, tenant-owner transfer, seat/member limits, invitation lifecycle,
settings ownership, audit requirements, and deletion behavior. Then add
migrations, idempotent handlers, durable audit, concurrency tests, and
dashboard/API flows.

This remains open because the current membership model is tenant-level and
fixed-role. Adding workspace membership or invitations without settled
inheritance and revocation semantics risks privilege escalation and
inconsistent ownership across the API and mailbox services.

## Acceptance criteria

- Tenant ownership remains unambiguous and transfer is atomic, audited, and
  recoverable.
- Workspace IDs cannot be used to cross tenant boundaries.
- Invitations are single-use, time-bounded, revocable, and safe under
  concurrent acceptance.
- Removing or disabling a member revokes access through API keys, sessions,
  workspace roles, and mailbox grants.
- Resource lists, aggregate counts, exports, and audit views are tenant- and
  workspace-scoped consistently.

## Research references

- [Control-plane schema](../../smtp-storage/postgres/000004_create_control_plane.up.sql)
- [Mailbox tenant constraints](../../smtp-storage/postgres/000008_mailbox_platform.up.sql)
- [Tenant isolation strategy](phase-72-multi-tenant-database.md)
