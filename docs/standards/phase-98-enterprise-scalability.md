# Phase 98 — Enterprise Scalability

Status: **Architecture targets documented; high availability, autoscaling, and failover are not production-validated**

## Repository baseline

The repository describes a future multi-instance deployment with managed
PostgreSQL, isolated API/SMTP/worker pools, object storage, secret management,
and telemetry. Current validation is local; performance evidence is a
sequential smoke baseline. There is no selected production provider,
autoscaling configuration, distributed cache, sharding, or tested
multi-region failover.

## Recommended scaling strategy

Measure the current architecture before adding distributed systems. First
establish SLOs, load profiles, tenant skew, queue lag, database pool/lock
pressure, storage growth, and SMTP connection behavior. Scale stateless API
and protocol instances horizontally; isolate resource pools and destination
bulkheads. Use PostgreSQL HA/read replicas only with explicit consistency and
failover semantics. Add cache only for measured bottlenecks with tenant-scoped
keys and invalidation. Shard or introduce multi-region writes only after a
routing, migration, idempotency, backup, residency, and recovery design.

Zero-downtime deployments require backward-compatible expand/migrate/contract
database changes, mixed-version API compatibility, queue/event schema
compatibility, readiness/drain behavior, safe worker leases, and tested
rollback. Auto-scaling must not overload PostgreSQL or remote MX destinations.

## Remaining work and why

Select provider/runtime and define availability, latency, throughput, RPO/RTO,
residency, and tenant capacity goals. Build representative concurrent and
failure workloads; instrument saturation and queue lag; implement
multi-instance deployment, connection budgets, graceful draining, backup/PITR,
and failover/restore exercises. Add distributed cache, sharding, or
multi-region only when evidence and operating ownership exist.

This remains open because there is no production deployment or workload
evidence. Autoscaling, caching, sharding, and failover are not safe
configuration toggles: each changes consistency and failure modes and requires
operational drills.

## Acceptance criteria

- Explicit SLO/SLI and load profiles include tenant skew, bursts, slow remote
  MX, retries, and dependency outages.
- Capacity tests identify saturation points and preserve isolation across
  tenants and delivery destinations.
- Failover, restore, rollback, rolling upgrade, and worker-drain procedures
  meet measured RPO/RTO.
- Database and downstream limits remain protected during autoscaling.
- Cache/shard/multi-region decisions are evidence-backed and include
  consistency, invalidation, residency, and failure tests.

## Research references

- [Repository deployment topology](../architecture/phase-01/data-and-deployment.md)
- [Phase 66 Backup and Recovery](phase-66-backup-and-recovery.md)
- [Google SRE Workbook](https://sre.google/workbook/table-of-contents/)
