# Phase 42 — User Mailboxes

Status: **Schema foundation implemented; lifecycle/API integration pending**

## Contract

User mailboxes are owned by a tenant and verified domain and may reference an
owner who is a tenant member. Shared mailboxes use the same mailbox record
with a `shared` kind and explicit user grants. Aliases resolve inbound
recipient addresses to the mailbox; they do not create copies or forward
messages by themselves.

Provisioning is one transaction: insert in `provisioning`, create required
default folders, record the quota, set `provisioned_at`, then activate. A
partial failure rolls back. Retry must be idempotent. Address uniqueness is
domain-scoped and case-insensitive for aliases; canonical domain handling
remains the responsibility of the verified-domain service. Address changes
and removal are audited.

Every operation must check tenant ownership, actor membership, role, and
mailbox-specific grant before reading or mutating state. Shared grants have
three levels: `read`, `read_write`, and `manage`. A tenant admin does not
silently obtain read access to message content. Revoking a user membership
also revokes mailbox grants.

Lifecycle changes are state transitions, not direct hard deletes. Suspension
blocks new delivery and user access according to product policy but retains
data. Deletion starts a tombstone and cleanup workflow subject to retention
and legal hold.

## Code and gaps

Migration `000008` provides mailbox kind, owner, quota, status, aliases, and
mailbox-scoped permission tables with same-tenant foreign keys. Rust
`MailboxStore` now provides transactional mailbox creation with default-folder
provisioning, lifecycle transitions, alias creation, and permission grants
and revocation. These are storage primitives, not an authenticated control
plane. Existing `GET /v1/mailboxes` remains backed by process-local Go state.
`mailbox_addresses` is the canonical case-insensitive routing registry for
primary addresses and aliases, preventing aliases from shadowing another
mailbox's primary address.
Durable REST handlers, idempotency, audit events, inbound routing, and
authorization integration are not implemented and must not be presented as
live service behavior.

## Acceptance criteria

- Concurrent duplicate mailbox/alias requests produce one canonical resource
  and a deterministic conflict response.
- Cross-tenant domain, owner, and grant assignments fail at the database
  boundary.
- Provisioning retries converge to one active mailbox and one default-folder
  set; injected failures leave no active partial mailbox.
- Membership revocation removes effective mailbox access immediately.
- Suspension and deletion preserve auditability and honor retention policy.
- API tests prove tenant isolation for both existing and non-existing IDs.
