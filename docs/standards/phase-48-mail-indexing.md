# Phase 48 — Mail Indexing

Status: **PostgreSQL index schema implemented; asynchronous projection population pending**

## Index contract

Only parsed, authorized, non-quarantined messages are indexed. Search rows
must remain mailbox- and tenant-scoped. The system-of-record message remains
valid if the search projection is delayed or rebuilt.

Migration `000009` adds parsed sender, recipients, body text, attachment names,
normalized text projections, and a weighted PostgreSQL `tsvector`. GIN supports full-text and array lookups;
partial B-tree indexes support live-folder date/sender/message-ID operations;
`pg_trgm` indexes support subject/sender similarity. Deleted messages are
excluded from the high-volume indexes. Index state is recorded by mailbox for
repair/rebuild progress.

Indexes have different jobs: folder and date queries use ordered B-tree keys;
message identifiers and sender use selective B-tree keys; recipient arrays
use GIN; text uses GIN `tsvector`; typo-tolerant matching uses trigram GIN.
No index should be added without an `EXPLAIN (ANALYZE, BUFFERS)` workload and
write-amplification review.

## Current implementation boundary

The migration creates structures, and the durable search function queries
them with tenant/mailbox predicates. `MailboxStore::append` does not yet parse
and populate these search columns, so documents must not claim messages are
automatically searchable. Projection writes need an outbox/worker, idempotent
upsert, delete propagation, replay cursor, dead-letter handling, and metrics.

**Why this remains open:** Index tables and a query function do not make new
messages searchable unless a reliable worker populates and updates projections.
Parser integration, replay, delete handling, and repair are prerequisites for
fresh and recoverable search results.

## Acceptance criteria

- Every indexed field maps to a parser output and canonical message version.
- Rebuild and incremental replay converge to identical search results.
- Tenant/mailbox restrictions are in the query plan and integration tests.
- Index size, write amplification, reindex lock impact, lag, and query latency
  are measured under a declared workload.
