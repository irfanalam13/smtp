# Phase 113 — Database Partitioning and Sharding

Status: **No partitioned or sharded deployment; data placement design pending**

## Goal and scope

Define how mailbox, tenant, message, and event data can grow beyond a single
database while preserving authorization boundaries, query behavior, retention,
backup, and recovery. Partitioning within a database and distributing data
across shards are different operational choices and must not be conflated.

## Repository baseline

The control plane and queue use PostgreSQL. The current schema and API are not
implemented as a multi-shard router, and no production dataset or measured
single-database saturation point is available. Phase 108 stateful storage HA
and Phases 72 and 98 have unresolved prerequisites.

## Recommended design

First measure table growth, hot indexes, tenant skew, write amplification,
vacuum/maintenance, and query plans. Prefer native partitioning for bounded
retention or predictable time/key access patterns when it materially improves
maintenance or query performance. A shard design must define a stable routing
key, tenant placement, resharding, cross-shard query limits, transactions,
idempotency, uniqueness scope, migration, backups, restore, and residency.

Keep authorization checks attached to the authoritative tenant and object
identifiers. Treat shard routing as placement only; it is not an authorization
decision. Plan for tenant moves and hot tenants without silently splitting
mailbox ordering or queue lease semantics.

## Remaining work and why

Gather production-like data and query profiles, select partition keys and
retention constraints, and test migration/restore/resharding. Do not introduce
shards based on a target user count alone; sharding adds distributed failure,
consistency, and operational burdens that the current code does not address.

## Acceptance criteria

- A documented capacity threshold and evidence justify each partition/shard.
- Migration and rollback work with mixed application versions and no lost mail.
- Tenant moves, uniqueness constraints, idempotency, and cross-shard operations
  have explicit semantics and tests.
- Backup, restore, retention, privacy deletion, and residency are validated
  for every placement unit.
- Hot-key and skew tests demonstrate bounded impact on other tenants.

## Research references

- [PostgreSQL table partitioning](https://www.postgresql.org/docs/current/ddl-partitioning.html)
- [Phase 72 Multi-Tenant Database](phase-72-multi-tenant-database.md)
- [Phase 108 Stateful Storage High Availability](phase-108-stateful-storage-ha.md)
