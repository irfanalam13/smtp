# Phase 41 — Mailbox Platform Architecture

Status: **Architecture defined; production service integration pending**

## Research baseline

- [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322) defines Internet message
  headers and message identifiers.
- [RFC 9051](https://www.rfc-editor.org/rfc/rfc9051) defines IMAP4rev2
  mailbox operations, UIDs, UIDVALIDITY, and message state.
- [RFC 6154](https://www.rfc-editor.org/rfc/rfc6154) defines IMAP special-use
  mailbox attributes.
- [RFC 9208](https://www.rfc-editor.org/rfc/rfc9208) defines the IMAP QUOTA
  extension.
- [RFC 2045](https://www.rfc-editor.org/rfc/rfc2045),
  [RFC 2046](https://www.rfc-editor.org/rfc/rfc2046), and
  [RFC 2231](https://www.rfc-editor.org/rfc/rfc2231) inform MIME and
  attachment handling.

These documents define protocol behavior; they do not prescribe a storage
vendor or imply the platform is currently interoperable or production-ready.

## Domain and ownership model

The ownership tree is `tenant → verified domain → mailbox → folders →
messages`. A mailbox has exactly one tenant and one domain. Its optional owner
must be a user in that tenant. A user mailbox is normally assigned to one
owner; a shared mailbox is accessed through explicit mailbox permissions.
Tenant membership alone does not grant access to message content.

An alias is a routing address that resolves to one mailbox. It is not a second
copy of a mailbox or message. Alias and mailbox creation must verify the
domain belongs to the same tenant. Shared mailbox permissions are
mailbox-scoped (`read`, `read_write`, `manage`) and revoked when tenant
membership ends.
Primary addresses and aliases share one canonical address registry so an
alias cannot shadow a mailbox's primary address.

Mailbox state is `provisioning → active → suspended → active → deleting`.
Provisioning, default-folder creation, initial quota setup, and activation
must commit atomically. Deletion is a tombstoned, asynchronous lifecycle so
retention, legal hold, backups, and object cleanup can be honored. Disabled
is retained as a compatibility state for existing mailbox rows.

## Persistence and scaling

PostgreSQL remains the source of truth for mailbox identity, access policy,
folder hierarchy, IMAP UID allocation, quota accounting, and searchable
message metadata. Every customer-owned row is tenant-bound, and cross-tenant
mailbox/domain references are rejected by relational constraints.

The current message engine stores raw RFC 5322 bytes in PostgreSQL `BYTEA`.
This is a bounded development implementation, not a large-object strategy.
The production target is immutable encrypted object storage behind an
`ObjectStore` interface, with small transactional metadata rows containing
tenant, object key, size, content hash, and lifecycle state. A content hash is
for integrity and same-tenant deduplication only; cross-tenant deduplication
is disabled by default to avoid existence and quota side channels. Object
writes use staged upload, database commit, and asynchronous orphan cleanup.

Scale-out proceeds by measuring and partitioning tenant/mailbox traffic,
partitioning large append-only message/change data, and routing mailbox
operations by stable mailbox ID. Search indexes and conversation projections
are rebuildable and never become the authorization source. Cache entries are
tenant- and mailbox-scoped and may not bypass current permission checks.

Quota is enforced in the same transaction as message append. The implemented
counter measures raw RFC message bytes; it does not yet include object-store
replication, indexes, backups, or provider billing units. The platform must
name that unit in APIs and IMAP quota reporting. The schema's initial 1 GiB
quota default is a bootstrap value only; tenant-plan policy and customer
limits must replace it before exposing mailbox provisioning publicly.

## API contract target

The durable API is tenant-scoped and requires authorization on every request:

- `GET/POST /v1/mailboxes`; `GET/PATCH/DELETE /v1/mailboxes/{mailbox_id}`
- `GET/POST/DELETE /v1/mailboxes/{mailbox_id}/aliases`
- `GET/PUT/DELETE /v1/mailboxes/{mailbox_id}/permissions/{user_id}`
- `GET/POST /v1/mailboxes/{mailbox_id}/folders`
- `GET/PATCH/DELETE /v1/mailboxes/{mailbox_id}/folders/{folder_id}`
- `GET /v1/mailboxes/{mailbox_id}/quota`
- Message append/fetch/search endpoints are specified with the message and
  synchronization phases, with cursor pagination and idempotency semantics.

Responses must not reveal whether a resource exists outside the caller's
tenant. Mutations need request IDs, idempotency where retried, audit events,
bounded payloads, and stable error codes. The current Go control-plane routes
  for mailbox listing and bounded message reads now use PostgreSQL, but the
  remaining target routes and mailbox write operations are not yet implemented.

## Implemented foundation and remaining work

Migration `000008` adds tenant/domain-scoped aliases and permissions, mailbox
kind/owner/quota/lifecycle metadata, folder hierarchy and UID allocation, and
message metadata fields. The Rust store now enforces atomic append quota
accounting and monotonically allocated folder UIDs. The Go API also serves
mailbox listing and bounded message metadata reads from PostgreSQL, with
user-bound API-key authentication and tenant/mailbox authorization.

Still required before production: authenticated mailbox lifecycle/write APIs,
idempotent provisioning, object storage, encryption/key rotation, disaster
recovery and restore tests, tenant-aware capacity planning, partitioning
evidence, abuse/retention policy, and load/failure testing. “Production scale”
is a validation target, not a capability claimed by this foundation.

**Why this remains open:** The durable read path is only one part of the target
architecture. Writes, message-byte lifecycle, key management, and operational
recovery have not been integrated and validated end to end; the current
PostgreSQL `BYTEA` implementation and isolated tests do not establish safe
production scale or recoverability.
