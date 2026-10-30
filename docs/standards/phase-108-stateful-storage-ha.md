# Phase 108 — Stateful Storage High Availability

Status: **PostgreSQL development foundation; managed database, Redis, and object-store HA are not deployed**

## Repository baseline

PostgreSQL migrations and repositories exist for selected API/mailbox/queue
features. Local development uses a single PostgreSQL container. There is no
selected managed HA PostgreSQL, Redis deployment, production object storage,
multi-zone failover, or measured recovery objective. Redis is not currently a
required dependency.

## Recommended design

Select stateful services from measured requirements. For PostgreSQL, choose
replication/failover semantics with explicit RPO, RTO, commit durability,
split-brain prevention, connection recovery, and replica consistency. Use
point-in-time recovery and regularly restore into an isolated environment.
Adopt Redis only for a defined cache/coordination need with tested loss and
rebuild behavior; it must not become an accidental source of truth. Object
storage must use encryption, versioning/retention as required, tenant
ownership, lifecycle policies, and restore/deletion verification.

## Remaining work and why

Choose providers, data durability and residency requirements, backup/PITR,
replication mode, failover manager, connection pool behavior, object
consistency, and key management. Execute planned failover, restore, corruption,
and provider-outage drills. This remains open because there is no selected
production provider, data-classification decision, or tested RPO/RTO.

## Acceptance criteria

- Database failover behavior and potential data loss are documented and
  measured under the selected replication mode.
- Backups are encrypted, access-controlled, monitored, and restored on a
  recurring schedule.
- Object data, metadata, keys, retention, and deletion remain consistent
  through failover/restore.
- Any cache loss degrades safely and cannot bypass tenant authorization.
- Recovery drills meet approved RPO/RTO and produce retained evidence.

## Research references

- [PostgreSQL High Availability, Load Balancing, and Replication](https://www.postgresql.org/docs/current/high-availability.html)
- [Phase 66 Backup and Recovery](phase-66-backup-and-recovery.md)
- [Data and deployment ownership](../architecture/phase-01/data-and-deployment.md)
