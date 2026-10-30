# Phase 112 — Distributed Caching

Status: **No distributed cache implementation; use cases and consistency contracts pending**

## Goal and scope

Introduce caching only for measured read bottlenecks, with tenant-scoped keys,
explicit freshness and invalidation semantics, bounded memory, and safe
behavior during cache loss. Cache availability must never become the source of
truth for mailbox, identity, authorization, billing, or delivery state.

## Repository baseline

The repository does not currently use Redis or another shared cache. The API
has process-local DNS and rate-limit state, which does not coordinate across
replicas and must not be interpreted as a global quota or abuse-prevention
control.

## Recommended design

For each candidate, identify the authoritative store, read/write path, maximum
staleness, invalidation trigger, tenant boundary, eviction policy, and
cache-outage behavior. Use namespaced tenant keys and avoid caching secrets,
raw message bodies, authentication decisions, or mutable authorization state
unless a reviewed invalidation contract makes that safe. Use bounded TTLs,
request coalescing where needed, and stampede protection that does not create
unbounded work during recovery.

Cache failures should degrade to an explicitly budgeted source-of-truth path,
not return stale success-shaped data. Protect PostgreSQL with admission
control, bounded retries, and load shedding. Do not cache writes in a way that
can acknowledge data before durable persistence.

## Remaining work and why

Collect profiles that show the database or another read dependency is the
bottleneck. Select and operate a cache only after evaluating tenant isolation,
consistency, persistence, failover, cost, and data residency. This remains
open because no shared cache or operational evidence exists, and deploying
one without measured demand adds a new failure and consistency boundary.

## Acceptance criteria

- Cache hit/miss, eviction, stale-read, latency, and backend saturation are
  observable without tenant- or key-cardinality labels.
- Cache failures and cold starts are fault-tested with explicit fallback limits.
- Cross-tenant key collision/isolation tests are required before rollout.
- Authorization, billing, mailbox writes, and queue state remain authoritative
  in durable stores.
- Cache invalidation and schema/version migration are documented and tested.

## Research references

- [Phase 98 Enterprise Scalability](phase-98-enterprise-scalability.md)
- [Google SRE Workbook](https://sre.google/workbook/table-of-contents/)
