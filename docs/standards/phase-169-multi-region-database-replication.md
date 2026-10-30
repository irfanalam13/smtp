# Phase 169 — Multi-Region Database Replication

Status: **Single local PostgreSQL; no managed replication or write topology**

## Recommended design

Keep one authoritative writer initially and select managed cross-region
replication or a documented PostgreSQL architecture only after measuring
consistency and recovery needs. Define lag, promotion, split-brain prevention,
transaction/idempotency behavior, sequence/uniqueness scope, read freshness,
and failback. Mail delivery has external side effects; database rollback or
replay must not create duplicate SMTP delivery or lose queue ownership.

## Remaining work and why

Approve the database provider, regions, RPO/RTO, tenant placement, workload,
and recovery operators. This remains open because schemas and services are
not yet deployed in a production managed database, and multi-writer conflicts
are unresolved.

## Acceptance criteria

- Writer ownership is fenced and cannot split-brain.
- Replication lag and failover data loss are measured and within approved
  objectives.
- Queue leases, idempotency, audit, identity, and mailbox writes recover
  consistently.
- Failback/reconciliation is tested on isolated data.
- Regional read replicas do not bypass authorization or serve prohibited stale
  data.

## Research references

- [PostgreSQL high availability](https://www.postgresql.org/docs/current/high-availability.html)
- [Phase 108 Stateful Storage HA](phase-108-stateful-storage-ha.md)
- [Phase 123 Point-in-Time Recovery](phase-123-point-in-time-recovery.md)
