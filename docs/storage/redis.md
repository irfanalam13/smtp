# Redis policy

Redis is intentionally not provisioned in Phase 06. Phase 01 did not approve a
current Redis consumer, and no service currently has a cache, rate limiter, or
ephemeral coordination contract that requires it.

When a real consumer is approved, it must document key ownership, bounded
environment/service/category keys, TTL for every ephemeral key, cache-aside
behavior, PostgreSQL source of truth, outage behavior, stale-read policy,
eviction policy, persistence, restoration, and atomic operations.

Adding Redis to Compose without an executable consumer would create operational
cost and misleading durability assumptions, so it remains deferred.
