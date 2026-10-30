# Phase 50 — Search Optimization

Status: **Index and query foundations in place; operational optimization pending**

## Optimization approach

Start with PostgreSQL full-text, trigram, and B-tree indexes on the same
source-of-truth cluster. Capture representative plans and workload
measurements before introducing another search service. Maintain generated
search vectors from normalized projections, not from raw MIME bytes in a
request handler.

Incremental projection updates should be idempotent and replayable from a
transactional outbox/change log. Reindexing must be throttled, observable,
resumable, and isolated from user-facing query capacity. Cache only bounded,
tenant/mailbox-scoped results with authorization checks before serving them;
invalidate or version cache entries on message, flag, permission, and deletion
changes. Never cache access decisions as durable authorization.

Distributed search is a future architecture decision triggered by measured
PostgreSQL limits. It requires a partition/routing plan, event lag and
replay guarantees, per-tenant isolation, index rebuild, split-brain handling,
query fan-out limits, and documented consistency semantics.

## Implemented code and gaps

Migration `000009` adds partial/live indexes, weighted full-text GIN,
trigram sender/subject indexes, recipient GIN, and per-mailbox index cursor
state. The query function has a bounded limit and offset. There is no
projection worker, result cache, query plan corpus, incremental update
pipeline, distributed backend, or observed performance baseline yet.

**Why this remains open:** There is not yet a representative production corpus,
tenant distribution, or latency target against which to justify caches,
reindex throttles, or distributed search. Optimization should follow measured
query plans and load evidence to avoid adding complexity without demonstrated
benefit.

## Acceptance criteria

- `EXPLAIN ANALYZE` shows expected indexes under realistic data distributions.
- Concurrent writes and reindexing do not block mailbox mutations beyond
  agreed budgets.
- Cache invalidation, stale index behavior, replay, and backend failure are
  tested.
- Benchmark reports include corpus size, tenant skew, concurrency, p50/p95/p99
  latency, throughput, freshness lag, index size, and resource usage.
- A distributed search backend is adopted only after PostgreSQL limits are
  reproduced and an ADR compares consistency, cost, and operational risk.

## References

- [PostgreSQL full-text search](https://www.postgresql.org/docs/current/textsearch.html)
- [PostgreSQL text-search functions](https://www.postgresql.org/docs/current/functions-textsearch.html)
- [PostgreSQL `pg_trgm`](https://www.postgresql.org/docs/current/pgtrgm.html)
