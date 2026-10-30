# Phase 170 — Distributed Cache

Status: **No Redis deployment or shared cache is implemented**

## Recommended design

Separate regional caches from any global coordination state. Cache only
rebuildable, non-authoritative data with tenant-scoped keys, bounded TTL,
explicit invalidation, and a safe miss/failure path. Never make cache state
authoritative for identity, authorization, mail durability, queue leases, or
global rate enforcement. Define data residency and regional eviction behavior.

## Remaining work and why

Measure cacheable bottlenecks, then select service, region placement,
consistency, failover, keying, and eviction. This remains open because Redis
has no current consumer and cache introduction would add a new availability
and privacy boundary.

## Acceptance criteria

- Cache outages/cold starts do not return unauthorized or stale success-shaped
  data.
- Tenant-boundary and invalidation tests run across regional cache instances.
- Cache replication and residency are explicit; no sensitive global cache by
  default.
- Database protection remains bounded during cache loss.
- Metrics avoid keys and customer identifiers.

## Research references

- [Phase 112 Distributed Caching](phase-112-distributed-caching.md)
- [Phase 108 Stateful Storage HA](phase-108-stateful-storage-ha.md)
