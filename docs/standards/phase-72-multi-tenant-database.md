# Phase 72 — Multi-Tenant Database

Status: **Shared PostgreSQL isolation strategy recommended; one inbound ownership constraint implemented; full audit pending**

## Repository baseline

PostgreSQL is the authoritative store. Existing control-plane and mailbox
tables include `tenant_id`, tenant-scoped indexes, and composite foreign keys
that prevent selected cross-tenant references. API queries derive tenant scope
from an authenticated principal and use parameterized SQL. PostgreSQL RLS is
not yet configured for the Go application role across all tenant-owned tables.
The current data includes mailbox content, credentials, delivery metadata,
audit records, indexes, and outbox/synchronization state.

## Recommended isolation strategy

Use a shared PostgreSQL database with explicit `tenant_id` on every
tenant-owned row as the initial default. Enforce isolation in application
authorization and query predicates, composite tenant-aware foreign keys,
unique constraints, indexes, and negative cross-tenant tests. Evaluate RLS as
additional defense in depth only after connection-pool transaction context,
application roles, migration ownership, and every relevant policy are
designed and tested.

Do not use schema-per-tenant as the default: it increases migration, catalog,
pooling, backup, and operational complexity without removing the need for
application authorization. Keep a future dedicated-database option for
customer-specific regulatory, residency, noisy-neighbor, or measured capacity
requirements. A routing/catalog layer and per-tenant restore/failover plan are
prerequisites for that option.

For RLS, application connections must use a non-owner role without
`BYPASSRLS`; tenant context must be set transaction-locally and reset by
transaction completion so pooled connections cannot leak context. RLS must
fail closed when context is absent. RLS does not replace query scoping,
authorization, database privileges, or tests.

## Remaining work and why

Inventory all tenant-owned tables, views, functions, object keys, caches,
events, queues, search projections, and backups. Add tenant-composite
constraints where missing; validate indexes and uniqueness; design least-
privilege roles; decide whether and where to enable RLS; add isolation tests
for reads, writes, joins, aggregates, background workers, and connection-pool
reuse. Define migration rollout, backup, restore, and any dedicated-tenant
routing plan.

This remains open because current tenant safety is implemented at selected
application and relational boundaries, not comprehensively across every data
path. Enabling RLS casually can break owner-driven migrations or create false
confidence if the application role bypasses policies or pooled session state
is reused.

**Initial implementation:** Migration `000014` adds a composite foreign key
from inbound message `(mailbox_id, tenant_id)` to the owning mailbox and
refuses to proceed if inconsistent existing rows are found. An isolated
PostgreSQL integration test verifies same-tenant inserts succeed and
cross-tenant references fail. This is one database-boundary fix, not a
complete tenant-isolation audit.

## Acceptance criteria

- Every tenant-owned row and derived artifact has an explicit tenant owner or
  a documented safe global classification.
- Cross-tenant reads and mutations fail in integration tests, including
  guessed IDs, joins, bulk operations, workers, and absent tenant context.
- Constraints prevent cross-tenant references even if application code has a
  bug.
- RLS, if selected, is tested with the exact non-owner production role and
  pooled connections; absent context denies access.
- Migration and restore tests prove tenant data remains correctly isolated.

## Research references

- [PostgreSQL Row Security Policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [Tenant isolation ADR proposal](../architecture/adr/phase-01/0014-tenant-isolation.md)
- [Repository data ownership](../architecture/phase-01/data-and-deployment.md)
- [Phase 71 SaaS architecture](phase-71-saas-architecture.md)
